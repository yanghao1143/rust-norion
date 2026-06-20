use crate::aggregate::AggregationConflictReviewTrendGateDecision;
use crate::budget::AgentBudget;
use crate::cycle::AgentCycleDispatch;
use crate::cycle::AgentCycleHandoff;
use crate::execute::AgentWaveExecution;
use crate::ports::{MemoryNote, MemoryPort, MemoryRecord};
use crate::reflection::ReflectionLoopHistoryGateDecision;
use crate::task::{AgentRole, AgentTask};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryRecallPolicy {
    pub limit_per_task: usize,
    pub max_context_records_per_task: usize,
    pub max_summary_chars: usize,
}

impl Default for MemoryRecallPolicy {
    fn default() -> Self {
        Self {
            limit_per_task: 4,
            max_context_records_per_task: 4,
            max_summary_chars: 240,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRecallDecisionKind {
    Admit,
    RejectBudget,
    RejectDuplicate,
    RejectEmptyId,
    RejectEmptySummary,
    RejectUnsafeSidecar,
}

impl MemoryRecallDecisionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Admit => "admit",
            Self::RejectBudget => "reject_budget",
            Self::RejectDuplicate => "reject_duplicate",
            Self::RejectEmptyId => "reject_empty_id",
            Self::RejectEmptySummary => "reject_empty_summary",
            Self::RejectUnsafeSidecar => "reject_unsafe_sidecar",
        }
    }

    pub fn accepted(self) -> bool {
        self == Self::Admit
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryRecallDryRunEvidence {
    pub source: String,
    pub read_only: bool,
    pub candidate_count: usize,
    pub long_term_match_count: usize,
    pub context_decision_count: usize,
    pub accepted_context_count: usize,
    pub rejected_context_count: usize,
    pub used_tokens: usize,
    pub requested_kv_count: usize,
    pub kv_promote_count: usize,
    pub kv_missing_count: usize,
    pub kv_already_hot_count: usize,
    pub kv_duplicate_count: usize,
    pub kv_backend_available: bool,
    pub memory_store_write_allowed: bool,
    pub kv_prefetch_apply_allowed: bool,
    pub reason_codes: Vec<String>,
    pub detail_codes: Vec<String>,
}

impl MemoryRecallDryRunEvidence {
    pub fn safe_for_recall_sidecar(&self) -> bool {
        self.safety_reasons().is_empty()
    }

    pub fn safety_reasons(&self) -> Vec<String> {
        let mut reasons = Vec::new();
        if !self.read_only {
            reasons.push("dry_run_not_read_only".to_owned());
        }
        if self.memory_store_write_allowed {
            reasons.push("memory_store_write_allowed".to_owned());
        }
        if self.kv_prefetch_apply_allowed {
            reasons.push("kv_prefetch_apply_allowed".to_owned());
        }
        reasons
    }

    pub fn record_id(&self, task_id: &str) -> String {
        format!(
            "memory-reuse-dry-run:{}:{}",
            hex_id(&self.source),
            hex_id(task_id)
        )
    }

    pub fn summary_text(&self) -> String {
        format!(
            "source={} read_only={} candidates={} long_term_matches={} context_decisions={} context_accepted={} context_rejected={} used_tokens={} kv_requested={} kv_promote={} kv_missing={} kv_hot={} kv_duplicate={} kv_backend_available={} memory_store_write_allowed={} kv_prefetch_apply_allowed={} reason_codes={} detail_codes={}",
            normalized_source(&self.source),
            self.read_only,
            self.candidate_count,
            self.long_term_match_count,
            self.context_decision_count,
            self.accepted_context_count,
            self.rejected_context_count,
            self.used_tokens,
            self.requested_kv_count,
            self.kv_promote_count,
            self.kv_missing_count,
            self.kv_already_hot_count,
            self.kv_duplicate_count,
            self.kv_backend_available,
            self.memory_store_write_allowed,
            self.kv_prefetch_apply_allowed,
            join_codes(self.reason_codes.clone()),
            join_codes(self.detail_codes.clone()),
        )
    }

    pub fn to_memory_record(&self, task_id: &str) -> MemoryRecord {
        MemoryRecord::new(
            self.record_id(task_id),
            self.summary_text(),
            normalized_source(&self.source),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryRecallItem {
    pub id: String,
    pub source: String,
    pub summary: String,
}

impl MemoryRecallItem {
    pub fn from_record(record: &MemoryRecord, max_summary_chars: usize) -> Self {
        Self {
            id: record.id.trim().to_owned(),
            source: normalized_source(&record.source),
            summary: compact_summary(&record.summary, max_summary_chars),
        }
    }

    pub fn context_line(&self) -> String {
        format!(
            "memory source={} id={} summary={}",
            self.source, self.id, self.summary
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryRecallDecision {
    pub record_id: String,
    pub kind: MemoryRecallDecisionKind,
    pub item: Option<MemoryRecallItem>,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryRecallContext {
    pub task_id: String,
    pub query: String,
    pub requested_limit: usize,
    pub returned_records: usize,
    pub read_only: bool,
    pub decisions: Vec<MemoryRecallDecision>,
    pub failure: Option<String>,
    pub telemetry: Vec<String>,
}

impl MemoryRecallContext {
    pub fn accepted_items(&self) -> Vec<&MemoryRecallItem> {
        self.decisions
            .iter()
            .filter_map(|decision| decision.item.as_ref())
            .collect()
    }

    pub fn accepted_count(&self) -> usize {
        self.accepted_items().len()
    }

    pub fn rejected_count(&self) -> usize {
        self.decisions
            .iter()
            .filter(|decision| !decision.kind.accepted())
            .count()
    }

    pub fn failed(&self) -> bool {
        self.failure.is_some()
    }

    pub fn context_lines(&self) -> Vec<String> {
        self.accepted_items()
            .into_iter()
            .map(MemoryRecallItem::context_line)
            .collect()
    }

    pub fn reason_codes(&self) -> Vec<String> {
        let mut codes = BTreeSet::new();
        codes.insert("read_only".to_owned());
        if self.failed() {
            codes.insert("recall_failed".to_owned());
        }
        if self.returned_records > 0 {
            codes.insert("records_returned".to_owned());
        }
        if self.accepted_count() > 0 {
            codes.insert("records_admitted".to_owned());
        }
        if self.rejected_count() > 0 {
            codes.insert("records_rejected".to_owned());
        }
        codes.extend(
            self.decisions
                .iter()
                .flat_map(|decision| decision.reasons.iter().cloned()),
        );
        codes.into_iter().collect()
    }

    pub fn detail_codes(&self) -> Vec<String> {
        let mut codes = BTreeSet::new();
        if self.failed() {
            codes.insert(format!("failure:{}", hex_id(&self.task_id)));
        }
        codes.extend(self.decisions.iter().flat_map(|decision| {
            decision.reasons.iter().map(move |reason| {
                format!(
                    "{}:{}:{}",
                    decision.kind.as_str(),
                    reason,
                    hex_id(&decision.record_id)
                )
            })
        }));
        codes.into_iter().collect()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "agent_memory_recall_context read_only={} task_id={} requested_limit={} returned={} admitted={} rejected={} failed={} reason_codes={} detail_codes={}",
            self.read_only,
            self.task_id,
            self.requested_limit,
            self.returned_records,
            self.accepted_count(),
            self.rejected_count(),
            self.failed(),
            join_codes(self.reason_codes()),
            join_codes(self.detail_codes()),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentWaveMemoryRecallPlan {
    pub contexts: Vec<MemoryRecallContext>,
    pub telemetry: Vec<String>,
}

impl AgentWaveMemoryRecallPlan {
    pub fn task_count(&self) -> usize {
        self.contexts.len()
    }

    pub fn accepted_count(&self) -> usize {
        self.contexts
            .iter()
            .map(MemoryRecallContext::accepted_count)
            .sum()
    }

    pub fn rejected_count(&self) -> usize {
        self.contexts
            .iter()
            .map(MemoryRecallContext::rejected_count)
            .sum()
    }

    pub fn failed_count(&self) -> usize {
        self.contexts
            .iter()
            .filter(|context| context.failed())
            .count()
    }

    pub fn read_only(&self) -> bool {
        self.contexts.iter().all(|context| context.read_only)
    }

    pub fn summary_line(&self) -> String {
        format!(
            "agent_wave_memory_recall read_only={} tasks={} admitted={} rejected={} failed={}",
            self.read_only(),
            self.task_count(),
            self.accepted_count(),
            self.rejected_count(),
            self.failed_count(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryRecallContextPlanner {
    pub policy: MemoryRecallPolicy,
}

impl Default for MemoryRecallContextPlanner {
    fn default() -> Self {
        Self {
            policy: MemoryRecallPolicy::default(),
        }
    }
}

impl MemoryRecallContextPlanner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_policy(mut self, policy: MemoryRecallPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn plan_for_task<P>(&self, task: &AgentTask, memory: &P) -> MemoryRecallContext
    where
        P: MemoryPort,
        P::Error: ToString,
    {
        let query = memory_recall_query(task);
        match memory.recall(&query, self.policy.limit_per_task.max(1)) {
            Ok(records) => self.plan_from_records(task, query, records),
            Err(error) => failed_memory_recall_context(
                task,
                query,
                self.policy.limit_per_task.max(1),
                error.to_string(),
            ),
        }
    }

    pub fn plan_from_records(
        &self,
        task: &AgentTask,
        query: String,
        records: Vec<MemoryRecord>,
    ) -> MemoryRecallContext {
        let mut decisions = Vec::new();
        let mut seen_ids = BTreeSet::new();
        let mut accepted = 0usize;

        for record in &records {
            let record_id = record.id.trim().to_owned();
            let summary = record.summary.trim();
            let (kind, item, reasons) = if record_id.is_empty() {
                (
                    MemoryRecallDecisionKind::RejectEmptyId,
                    None,
                    vec!["empty_id".to_owned()],
                )
            } else if summary.is_empty() {
                (
                    MemoryRecallDecisionKind::RejectEmptySummary,
                    None,
                    vec!["empty_summary".to_owned()],
                )
            } else if !seen_ids.insert(record_id.clone()) {
                (
                    MemoryRecallDecisionKind::RejectDuplicate,
                    None,
                    vec!["duplicate_id".to_owned()],
                )
            } else if accepted >= self.policy.max_context_records_per_task {
                (
                    MemoryRecallDecisionKind::RejectBudget,
                    None,
                    vec!["max_context_records".to_owned()],
                )
            } else {
                accepted = accepted.saturating_add(1);
                (
                    MemoryRecallDecisionKind::Admit,
                    Some(MemoryRecallItem::from_record(
                        record,
                        self.policy.max_summary_chars,
                    )),
                    vec!["admitted".to_owned()],
                )
            };

            decisions.push(MemoryRecallDecision {
                record_id,
                kind,
                item,
                reasons,
            });
        }

        let telemetry = memory_recall_context_telemetry(
            &task.id,
            records.len(),
            accepted,
            decisions.len().saturating_sub(accepted),
            false,
        );

        MemoryRecallContext {
            task_id: task.id.clone(),
            query,
            requested_limit: self.policy.limit_per_task.max(1),
            returned_records: records.len(),
            read_only: true,
            decisions,
            failure: None,
            telemetry,
        }
    }

    pub fn plan_from_dry_run_evidence(
        &self,
        task: &AgentTask,
        evidence: &MemoryRecallDryRunEvidence,
    ) -> MemoryRecallContext {
        let query = format!(
            "{} dry_run_source:{}",
            memory_recall_query(task),
            normalized_source(&evidence.source)
        );
        let safety_reasons = evidence.safety_reasons();
        if safety_reasons.is_empty() {
            let mut context =
                self.plan_from_records(task, query, vec![evidence.to_memory_record(&task.id)]);
            for decision in &mut context.decisions {
                if decision.kind.accepted() {
                    decision.reasons.push("dry_run_sidecar".to_owned());
                    decision.reasons.extend(
                        evidence
                            .reason_codes
                            .iter()
                            .map(|code| format!("dry_run:{code}")),
                    );
                }
            }
            context
                .telemetry
                .extend(memory_recall_dry_run_evidence_telemetry(
                    task, evidence, true, false,
                ));
            return context;
        }

        let mut reasons = vec!["dry_run_sidecar".to_owned()];
        reasons.extend(safety_reasons);
        let mut telemetry = memory_recall_context_telemetry(&task.id, 1, 0, 1, false);
        telemetry.extend(memory_recall_dry_run_evidence_telemetry(
            task, evidence, false, true,
        ));

        MemoryRecallContext {
            task_id: task.id.clone(),
            query,
            requested_limit: 1,
            returned_records: 1,
            read_only: true,
            decisions: vec![MemoryRecallDecision {
                record_id: evidence.record_id(&task.id),
                kind: MemoryRecallDecisionKind::RejectUnsafeSidecar,
                item: None,
                reasons,
            }],
            failure: None,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentWaveMemoryRecallPlanner {
    pub context_planner: MemoryRecallContextPlanner,
}

impl Default for AgentWaveMemoryRecallPlanner {
    fn default() -> Self {
        Self {
            context_planner: MemoryRecallContextPlanner::default(),
        }
    }
}

impl AgentWaveMemoryRecallPlanner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_context_planner(mut self, context_planner: MemoryRecallContextPlanner) -> Self {
        self.context_planner = context_planner;
        self
    }

    pub fn plan_dispatch<P>(
        &self,
        dispatch: &AgentCycleDispatch,
        memory: &P,
    ) -> AgentWaveMemoryRecallPlan
    where
        P: MemoryPort,
        P::Error: ToString,
    {
        let contexts = dispatch
            .assigned_tasks
            .iter()
            .map(|task| self.context_planner.plan_for_task(task, memory))
            .collect::<Vec<_>>();
        let telemetry = agent_wave_memory_recall_telemetry(&contexts);

        AgentWaveMemoryRecallPlan {
            contexts,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentRecallOutcomeAttributionPolicy {
    pub reinforce_amount: f32,
    pub rejected_penalty_amount: f32,
    pub failure_penalty_amount: f32,
}

impl Default for AgentRecallOutcomeAttributionPolicy {
    fn default() -> Self {
        Self {
            reinforce_amount: 0.24,
            rejected_penalty_amount: 0.18,
            failure_penalty_amount: 0.32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentRecallOutcomeAttributionAction {
    Reinforce,
    Penalize,
}

impl AgentRecallOutcomeAttributionAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Reinforce => "reinforce",
            Self::Penalize => "penalize",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRecallOutcomeAttribution {
    pub task_id: String,
    pub record_id: String,
    pub source: String,
    pub action: AgentRecallOutcomeAttributionAction,
    pub amount: f32,
    pub reason_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRecallOutcomeAttributionReport {
    pub attributions: Vec<AgentRecallOutcomeAttribution>,
    pub reinforced_count: usize,
    pub penalized_count: usize,
    pub skipped_rejected_recall_count: usize,
    pub skipped_missing_outcome_task_ids: Vec<String>,
    pub read_only: bool,
    pub memory_store_write_allowed: bool,
    pub telemetry: Vec<String>,
}

impl AgentRecallOutcomeAttributionReport {
    pub fn has_updates(&self) -> bool {
        !self.attributions.is_empty()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "agent_recall_outcome_attribution read_only={} memory_store_write_allowed={} updates={} reinforce={} penalize={} skipped_rejected_recall={} skipped_missing_outcome_tasks={}",
            self.read_only,
            self.memory_store_write_allowed,
            self.attributions.len(),
            self.reinforced_count,
            self.penalized_count,
            self.skipped_rejected_recall_count,
            self.skipped_missing_outcome_task_ids.len(),
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRecallOutcomeAttributionPlanner {
    pub policy: AgentRecallOutcomeAttributionPolicy,
}

impl Default for AgentRecallOutcomeAttributionPlanner {
    fn default() -> Self {
        Self {
            policy: AgentRecallOutcomeAttributionPolicy::default(),
        }
    }
}

impl AgentRecallOutcomeAttributionPlanner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_policy(mut self, policy: AgentRecallOutcomeAttributionPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn plan(
        &self,
        recall_plan: &AgentWaveMemoryRecallPlan,
        execution: &AgentWaveExecution,
    ) -> AgentRecallOutcomeAttributionReport {
        let mut attributions = Vec::new();
        let mut skipped_missing_outcome_task_ids = BTreeSet::new();
        let mut skipped_rejected_recall_count = 0usize;

        for context in &recall_plan.contexts {
            skipped_rejected_recall_count =
                skipped_rejected_recall_count.saturating_add(context.rejected_count());
            let Some(status) = recall_outcome_task_status(&context.task_id, execution) else {
                if context.accepted_count() > 0 {
                    skipped_missing_outcome_task_ids.insert(context.task_id.clone());
                }
                continue;
            };

            for decision in &context.decisions {
                let Some(item) = decision.item.as_ref() else {
                    continue;
                };
                if !decision.kind.accepted() {
                    continue;
                }

                let (action, amount, outcome_reason) = match status {
                    RecallOutcomeTaskStatus::Accepted => (
                        AgentRecallOutcomeAttributionAction::Reinforce,
                        attribution_amount(self.policy.reinforce_amount),
                        "result_accepted",
                    ),
                    RecallOutcomeTaskStatus::Rejected => (
                        AgentRecallOutcomeAttributionAction::Penalize,
                        attribution_amount(self.policy.rejected_penalty_amount),
                        "result_rejected",
                    ),
                    RecallOutcomeTaskStatus::Failed => (
                        AgentRecallOutcomeAttributionAction::Penalize,
                        attribution_amount(self.policy.failure_penalty_amount),
                        "execution_failed",
                    ),
                };

                let mut reason_codes = BTreeSet::new();
                reason_codes.insert("recall_admitted".to_owned());
                reason_codes.insert(outcome_reason.to_owned());
                if context.read_only {
                    reason_codes.insert("recall_read_only".to_owned());
                }
                reason_codes.extend(decision.reasons.iter().cloned());

                attributions.push(AgentRecallOutcomeAttribution {
                    task_id: context.task_id.clone(),
                    record_id: item.id.clone(),
                    source: item.source.clone(),
                    action,
                    amount,
                    reason_codes: reason_codes.into_iter().collect(),
                });
            }
        }

        let reinforced_count = attributions
            .iter()
            .filter(|attribution| {
                attribution.action == AgentRecallOutcomeAttributionAction::Reinforce
            })
            .count();
        let penalized_count = attributions
            .iter()
            .filter(|attribution| {
                attribution.action == AgentRecallOutcomeAttributionAction::Penalize
            })
            .count();
        let skipped_missing_outcome_task_ids = skipped_missing_outcome_task_ids
            .into_iter()
            .collect::<Vec<_>>();
        let telemetry = recall_outcome_attribution_telemetry(
            attributions.len(),
            reinforced_count,
            penalized_count,
            skipped_rejected_recall_count,
            skipped_missing_outcome_task_ids.len(),
        );

        AgentRecallOutcomeAttributionReport {
            attributions,
            reinforced_count,
            penalized_count,
            skipped_rejected_recall_count,
            skipped_missing_outcome_task_ids,
            read_only: true,
            memory_store_write_allowed: false,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentMemoryReuseExecutionPreflightPolicy {
    pub require_recall_for_each_task: bool,
    pub require_dry_run_evidence: bool,
    pub block_on_recall_failure: bool,
}

impl Default for AgentMemoryReuseExecutionPreflightPolicy {
    fn default() -> Self {
        Self {
            require_recall_for_each_task: true,
            require_dry_run_evidence: true,
            block_on_recall_failure: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMemoryReuseExecutionPreflightReport {
    pub task_ids: Vec<String>,
    pub recall_task_ids: Vec<String>,
    pub missing_recall_task_ids: Vec<String>,
    pub unexpected_recall_task_ids: Vec<String>,
    pub failed_recall_task_ids: Vec<String>,
    pub dry_run_evidence_sources: Vec<String>,
    pub unsafe_dry_run_sources: Vec<String>,
    pub read_only: bool,
    pub memory_reuse_ready: bool,
    pub can_enter_execution: bool,
    pub requires_repair_first: bool,
    pub prompt_injection_allowed: bool,
    pub engine_port_touched: bool,
    pub memory_store_write_allowed: bool,
    pub kv_prefetch_apply_allowed: bool,
    pub accepted_recall_count: usize,
    pub rejected_recall_count: usize,
    pub dry_run_evidence_count: usize,
    pub kv_requested_count: usize,
    pub kv_promote_count: usize,
    pub kv_missing_count: usize,
    pub kv_already_hot_count: usize,
    pub kv_duplicate_count: usize,
    pub blocked_reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl AgentMemoryReuseExecutionPreflightReport {
    pub fn is_clean(&self) -> bool {
        !self.requires_repair_first && self.blocked_reasons.is_empty()
    }

    pub fn reason_codes(&self) -> Vec<String> {
        let mut codes = BTreeSet::new();
        codes.insert("read_only".to_owned());
        if self.memory_reuse_ready {
            codes.insert("memory_reuse_ready".to_owned());
        }
        if self.can_enter_execution {
            codes.insert("can_enter_execution".to_owned());
        }
        if self.requires_repair_first {
            codes.insert("requires_repair_first".to_owned());
        }
        if !self.missing_recall_task_ids.is_empty() {
            codes.insert("missing_recall_tasks".to_owned());
        }
        if !self.unexpected_recall_task_ids.is_empty() {
            codes.insert("unexpected_recall_tasks".to_owned());
        }
        if !self.failed_recall_task_ids.is_empty() {
            codes.insert("failed_recall_tasks".to_owned());
        }
        if !self.unsafe_dry_run_sources.is_empty() {
            codes.insert("unsafe_dry_run_sources".to_owned());
        }
        if self.kv_requested_count > 0 {
            codes.insert("kv_prefetch_planned".to_owned());
        }
        codes.extend(self.blocked_reasons.iter().cloned());
        codes.into_iter().collect()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "agent_memory_reuse_execution_preflight read_only={} tasks={} recall_contexts={} dry_run_evidence={} memory_reuse_ready={} can_enter_execution={} requires_repair_first={} prompt_injection_allowed={} engine_port_touched={} memory_store_write_allowed={} kv_prefetch_apply_allowed={} accepted_recall={} rejected_recall={} kv_requested={} kv_promote={} kv_missing={} reason_codes={}",
            self.read_only,
            self.task_ids.len(),
            self.recall_task_ids.len(),
            self.dry_run_evidence_count,
            self.memory_reuse_ready,
            self.can_enter_execution,
            self.requires_repair_first,
            self.prompt_injection_allowed,
            self.engine_port_touched,
            self.memory_store_write_allowed,
            self.kv_prefetch_apply_allowed,
            self.accepted_recall_count,
            self.rejected_recall_count,
            self.kv_requested_count,
            self.kv_promote_count,
            self.kv_missing_count,
            join_codes(self.reason_codes()),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMemoryReuseExecutionPreflightPlanner {
    pub policy: AgentMemoryReuseExecutionPreflightPolicy,
}

impl Default for AgentMemoryReuseExecutionPreflightPlanner {
    fn default() -> Self {
        Self {
            policy: AgentMemoryReuseExecutionPreflightPolicy::default(),
        }
    }
}

impl AgentMemoryReuseExecutionPreflightPlanner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_policy(mut self, policy: AgentMemoryReuseExecutionPreflightPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn plan_for_dispatch(
        &self,
        dispatch: &AgentCycleDispatch,
        recall_plan: &AgentWaveMemoryRecallPlan,
        dry_run_evidence: &[MemoryRecallDryRunEvidence],
    ) -> AgentMemoryReuseExecutionPreflightReport {
        let task_ids = dispatch
            .assigned_tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        let task_id_set = task_ids.iter().cloned().collect::<BTreeSet<_>>();
        let recall_task_ids = recall_plan
            .contexts
            .iter()
            .map(|context| context.task_id.clone())
            .collect::<Vec<_>>();
        let recall_task_id_set = recall_task_ids.iter().cloned().collect::<BTreeSet<_>>();
        let missing_recall_task_ids = if self.policy.require_recall_for_each_task {
            task_id_set
                .difference(&recall_task_id_set)
                .cloned()
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let unexpected_recall_task_ids = recall_task_id_set
            .difference(&task_id_set)
            .cloned()
            .collect::<Vec<_>>();
        let failed_recall_task_ids = recall_plan
            .contexts
            .iter()
            .filter(|context| context.failed())
            .map(|context| context.task_id.clone())
            .collect::<Vec<_>>();
        let dry_run_evidence_sources = dry_run_evidence
            .iter()
            .map(|evidence| normalized_source(&evidence.source))
            .collect::<Vec<_>>();
        let unsafe_dry_run_sources = dry_run_evidence
            .iter()
            .filter(|evidence| !evidence.safe_for_recall_sidecar())
            .map(|evidence| normalized_source(&evidence.source))
            .collect::<Vec<_>>();
        let evidence_requested_memory_store_write = dry_run_evidence
            .iter()
            .any(|evidence| evidence.memory_store_write_allowed);
        let evidence_requested_kv_prefetch_apply = dry_run_evidence
            .iter()
            .any(|evidence| evidence.kv_prefetch_apply_allowed);
        let read_only = recall_plan.read_only()
            && dry_run_evidence.iter().all(|evidence| evidence.read_only)
            && !evidence_requested_memory_store_write
            && !evidence_requested_kv_prefetch_apply;
        let accepted_recall_count = recall_plan.accepted_count();
        let rejected_recall_count = recall_plan.rejected_count();
        let dry_run_evidence_count = dry_run_evidence.len();
        let kv_requested_count = dry_run_evidence
            .iter()
            .map(|evidence| evidence.requested_kv_count)
            .sum::<usize>();
        let kv_promote_count = dry_run_evidence
            .iter()
            .map(|evidence| evidence.kv_promote_count)
            .sum::<usize>();
        let kv_missing_count = dry_run_evidence
            .iter()
            .map(|evidence| evidence.kv_missing_count)
            .sum::<usize>();
        let kv_already_hot_count = dry_run_evidence
            .iter()
            .map(|evidence| evidence.kv_already_hot_count)
            .sum::<usize>();
        let kv_duplicate_count = dry_run_evidence
            .iter()
            .map(|evidence| evidence.kv_duplicate_count)
            .sum::<usize>();

        let mut blocked_reasons = Vec::new();
        if task_ids.is_empty() {
            blocked_reasons.push("dispatch_empty".to_owned());
        }
        if !read_only {
            blocked_reasons.push("memory_reuse_preflight_not_read_only".to_owned());
        }
        if self.policy.require_dry_run_evidence && dry_run_evidence.is_empty() {
            blocked_reasons.push("memory_reuse_dry_run_evidence_missing".to_owned());
        }
        if evidence_requested_memory_store_write {
            blocked_reasons.push("memory_store_write_allowed".to_owned());
        }
        if evidence_requested_kv_prefetch_apply {
            blocked_reasons.push("kv_prefetch_apply_allowed".to_owned());
        }
        blocked_reasons.extend(
            missing_recall_task_ids
                .iter()
                .map(|id| format!("memory_reuse_recall_missing_task={id}")),
        );
        blocked_reasons.extend(
            unexpected_recall_task_ids
                .iter()
                .map(|id| format!("memory_reuse_recall_unexpected_task={id}")),
        );
        if self.policy.block_on_recall_failure {
            blocked_reasons.extend(
                failed_recall_task_ids
                    .iter()
                    .map(|id| format!("memory_reuse_recall_failed_task={id}")),
            );
        }
        blocked_reasons.extend(
            unsafe_dry_run_sources
                .iter()
                .map(|source| format!("memory_reuse_dry_run_unsafe_source={source}")),
        );

        let requires_repair_first = !blocked_reasons.is_empty();
        let memory_reuse_ready = !requires_repair_first
            && read_only
            && (!self.policy.require_dry_run_evidence || dry_run_evidence_count > 0)
            && (accepted_recall_count > 0 || dry_run_evidence_count > 0);
        let can_enter_execution = !requires_repair_first && !task_ids.is_empty();
        let telemetry = memory_reuse_execution_preflight_telemetry(
            task_ids.len(),
            recall_task_ids.len(),
            dry_run_evidence_count,
            memory_reuse_ready,
            can_enter_execution,
            requires_repair_first,
            accepted_recall_count,
            rejected_recall_count,
            kv_requested_count,
            kv_promote_count,
            kv_missing_count,
        );

        AgentMemoryReuseExecutionPreflightReport {
            task_ids,
            recall_task_ids,
            missing_recall_task_ids,
            unexpected_recall_task_ids,
            failed_recall_task_ids,
            dry_run_evidence_sources,
            unsafe_dry_run_sources,
            read_only,
            memory_reuse_ready,
            can_enter_execution,
            requires_repair_first,
            prompt_injection_allowed: false,
            engine_port_touched: false,
            memory_store_write_allowed: false,
            kv_prefetch_apply_allowed: false,
            accepted_recall_count,
            rejected_recall_count,
            dry_run_evidence_count,
            kv_requested_count,
            kv_promote_count,
            kv_missing_count,
            kv_already_hot_count,
            kv_duplicate_count,
            blocked_reasons,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemorySubmissionReport {
    pub submitted: Vec<MemoryNote>,
    pub failures: Vec<MemorySubmissionFailure>,
    pub blocked_reasons: Vec<String>,
    pub note_quality: Option<MemoryNoteQualityReport>,
}

impl MemorySubmissionReport {
    pub fn is_clean(&self) -> bool {
        self.failures.is_empty() && self.blocked_reasons.is_empty()
    }

    pub fn summary(&self) -> MemorySubmissionSummary {
        MemorySubmissionSummary::from_report(self)
    }

    pub fn gate(&self) -> MemorySubmissionGateDecision {
        MemorySubmissionGateDecision::from_report(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemorySubmissionSummary {
    pub submitted_notes: usize,
    pub failed_notes: usize,
    pub blocked_reasons: usize,
    pub attempted_notes: usize,
    pub quality_reviewed_notes: usize,
    pub quality_admitted_notes: usize,
    pub quality_rejected_notes: usize,
    pub clean: bool,
    pub port_attempted: bool,
    pub telemetry: Vec<String>,
}

impl MemorySubmissionSummary {
    pub fn from_report(report: &MemorySubmissionReport) -> Self {
        let submitted_notes = report.submitted.len();
        let failed_notes = report.failures.len();
        let blocked_reasons = report.blocked_reasons.len();
        let attempted_notes = submitted_notes + failed_notes;
        let quality_reviewed_notes = report
            .note_quality
            .as_ref()
            .map(|quality| quality.decisions.len())
            .unwrap_or_default();
        let quality_admitted_notes = report
            .note_quality
            .as_ref()
            .map(|quality| quality.admitted_notes)
            .unwrap_or_default();
        let quality_rejected_notes = report
            .note_quality
            .as_ref()
            .map(|quality| quality.rejected_notes)
            .unwrap_or_default();
        let clean = report.is_clean();
        let port_attempted = attempted_notes > 0;
        let telemetry = memory_submission_summary_telemetry(
            submitted_notes,
            failed_notes,
            blocked_reasons,
            attempted_notes,
            quality_reviewed_notes,
            quality_admitted_notes,
            quality_rejected_notes,
            clean,
            port_attempted,
        );

        Self {
            submitted_notes,
            failed_notes,
            blocked_reasons,
            attempted_notes,
            quality_reviewed_notes,
            quality_admitted_notes,
            quality_rejected_notes,
            clean,
            port_attempted,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemorySubmissionGateDecision {
    pub summary: MemorySubmissionSummary,
    pub can_continue_loop: bool,
    pub can_commit_submitted_notes: bool,
    pub requires_repair_first: bool,
    pub reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemorySubmissionHealthStatus {
    Stable,
    Watch,
    Repair,
}

impl MemorySubmissionHealthStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Watch => "watch",
            Self::Repair => "repair",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MemorySubmissionSummaryHistory {
    summaries: Vec<MemorySubmissionSummary>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemorySubmissionDashboard {
    pub total_records: usize,
    pub clean_records: usize,
    pub repair_first_records: usize,
    pub submitted_notes: usize,
    pub failed_notes: usize,
    pub blocked_reasons: usize,
    pub attempted_notes: usize,
    pub quality_reviewed_notes: usize,
    pub quality_admitted_notes: usize,
    pub quality_rejected_notes: usize,
    pub port_attempted_records: usize,
    pub no_note_records: usize,
    pub clean_rate: f32,
    pub port_attempt_rate: f32,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemorySubmissionHealthPolicy {
    pub minimum_clean_rate: f32,
    pub minimum_port_attempt_rate: f32,
    pub maximum_failed_notes: usize,
    pub maximum_blocked_reasons: usize,
    pub maximum_no_note_records: usize,
}

impl Default for MemorySubmissionHealthPolicy {
    fn default() -> Self {
        Self {
            minimum_clean_rate: 0.67,
            minimum_port_attempt_rate: 0.0,
            maximum_failed_notes: 0,
            maximum_blocked_reasons: 0,
            maximum_no_note_records: usize::MAX,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemorySubmissionHealth {
    pub status: MemorySubmissionHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: MemorySubmissionDashboard,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemorySubmissionSummaryHistoryRecord {
    pub history: MemorySubmissionSummaryHistory,
    pub appended_summary: MemorySubmissionSummary,
    pub dashboard: MemorySubmissionDashboard,
    pub health: MemorySubmissionHealth,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct MemorySubmissionSummaryHistoryRecorder;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryNoteQualityDecisionKind {
    Admit,
    RejectEmptyTopic,
    RejectEmptyContent,
    RejectDuplicate,
}

impl MemoryNoteQualityDecisionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Admit => "admit",
            Self::RejectEmptyTopic => "reject_empty_topic",
            Self::RejectEmptyContent => "reject_empty_content",
            Self::RejectDuplicate => "reject_duplicate",
        }
    }

    pub fn accepted(self) -> bool {
        self == Self::Admit
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryNoteQualityDecision {
    pub index: usize,
    pub topic: String,
    pub kind: MemoryNoteQualityDecisionKind,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryNoteQualityReport {
    pub decisions: Vec<MemoryNoteQualityDecision>,
    pub admitted_notes: usize,
    pub rejected_notes: usize,
    pub telemetry: Vec<String>,
}

impl MemoryNoteQualityReport {
    pub fn from_notes(notes: &[MemoryNote]) -> Self {
        let mut decisions = Vec::new();
        let mut seen = BTreeSet::new();
        let mut admitted_notes = 0usize;

        for (index, note) in notes.iter().enumerate() {
            let topic = normalized_note_field(&note.topic);
            let content = normalized_note_field(&note.content);
            let fingerprint = format!(
                "{}\n{}",
                topic.to_ascii_lowercase(),
                content.to_ascii_lowercase()
            );
            let (kind, reasons) = if topic.is_empty() {
                (
                    MemoryNoteQualityDecisionKind::RejectEmptyTopic,
                    vec![format!("memory_note_quality_empty_topic index={index}")],
                )
            } else if content.is_empty() {
                (
                    MemoryNoteQualityDecisionKind::RejectEmptyContent,
                    vec![format!("memory_note_quality_empty_content index={index}")],
                )
            } else if !seen.insert(fingerprint) {
                (
                    MemoryNoteQualityDecisionKind::RejectDuplicate,
                    vec![format!("memory_note_quality_duplicate index={index}")],
                )
            } else {
                admitted_notes = admitted_notes.saturating_add(1);
                (
                    MemoryNoteQualityDecisionKind::Admit,
                    vec![format!("memory_note_quality_admitted index={index}")],
                )
            };

            decisions.push(MemoryNoteQualityDecision {
                index,
                topic,
                kind,
                reasons,
            });
        }

        let rejected_notes = decisions
            .iter()
            .filter(|decision| !decision.kind.accepted())
            .count();
        let telemetry =
            memory_note_quality_report_telemetry(notes.len(), admitted_notes, rejected_notes);

        Self {
            decisions,
            admitted_notes,
            rejected_notes,
            telemetry,
        }
    }

    pub fn rejection_reasons(&self) -> Vec<String> {
        self.decisions
            .iter()
            .filter(|decision| !decision.kind.accepted())
            .flat_map(|decision| decision.reasons.iter().cloned())
            .collect()
    }

    pub fn admitted_indexes(&self) -> Vec<usize> {
        self.decisions
            .iter()
            .filter(|decision| decision.kind.accepted())
            .map(|decision| decision.index)
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryPromotionGateDecision {
    pub candidate_notes: usize,
    pub admitted_candidate_notes: usize,
    pub rejected_candidate_notes: usize,
    pub note_quality: MemoryNoteQualityReport,
    pub reflection_gate: ReflectionLoopHistoryGateDecision,
    pub aggregation_conflict_gate: AggregationConflictReviewTrendGateDecision,
    pub memory_health: MemorySubmissionHealth,
    pub can_promote_memory_note: bool,
    pub can_submit_memory: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl MemoryPromotionGateDecision {
    pub fn is_memory_promotable(&self) -> bool {
        self.can_promote_memory_note && !self.requires_repair_first
    }
}

#[derive(Debug, Clone, Default)]
pub struct MemoryPromotionGate;

impl MemorySubmissionGateDecision {
    pub fn from_report(report: &MemorySubmissionReport) -> Self {
        let summary = report.summary();
        let mut reasons = Vec::new();

        reasons.extend(
            report
                .blocked_reasons
                .iter()
                .map(|reason| format!("memory_handoff_blocked reason={reason}")),
        );
        reasons.extend(report.failures.iter().map(|failure| {
            format!(
                "memory_submission_failed topic={} reason={}",
                failure.note.topic, failure.reason
            )
        }));

        let can_continue_loop = reasons.is_empty();
        let can_commit_submitted_notes = can_continue_loop && summary.submitted_notes > 0;
        let requires_repair_first = !can_continue_loop;
        let telemetry = memory_submission_gate_telemetry(
            can_continue_loop,
            can_commit_submitted_notes,
            requires_repair_first,
            reasons.len(),
            &summary,
        );

        Self {
            summary,
            can_continue_loop,
            can_commit_submitted_notes,
            requires_repair_first,
            reasons,
            telemetry,
        }
    }
}

impl MemorySubmissionSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<MemorySubmissionSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: MemorySubmissionSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&MemorySubmissionSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[MemorySubmissionSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> MemorySubmissionDashboard {
        MemorySubmissionDashboard::from_summaries(&self.summaries)
    }

    pub fn health(&self, policy: MemorySubmissionHealthPolicy) -> MemorySubmissionHealth {
        self.dashboard().health(policy)
    }
}

impl MemorySubmissionDashboard {
    pub fn from_summaries(summaries: &[MemorySubmissionSummary]) -> Self {
        let total_records = summaries.len();
        let clean_records = summaries.iter().filter(|summary| summary.clean).count();
        let repair_first_records = summaries.iter().filter(|summary| !summary.clean).count();
        let submitted_notes = summaries
            .iter()
            .map(|summary| summary.submitted_notes)
            .sum::<usize>();
        let failed_notes = summaries
            .iter()
            .map(|summary| summary.failed_notes)
            .sum::<usize>();
        let blocked_reasons = summaries
            .iter()
            .map(|summary| summary.blocked_reasons)
            .sum::<usize>();
        let attempted_notes = summaries
            .iter()
            .map(|summary| summary.attempted_notes)
            .sum::<usize>();
        let quality_reviewed_notes = summaries
            .iter()
            .map(|summary| summary.quality_reviewed_notes)
            .sum::<usize>();
        let quality_admitted_notes = summaries
            .iter()
            .map(|summary| summary.quality_admitted_notes)
            .sum::<usize>();
        let quality_rejected_notes = summaries
            .iter()
            .map(|summary| summary.quality_rejected_notes)
            .sum::<usize>();
        let port_attempted_records = summaries
            .iter()
            .filter(|summary| summary.port_attempted)
            .count();
        let no_note_records = summaries
            .iter()
            .filter(|summary| summary.submitted_notes == 0 && summary.attempted_notes == 0)
            .count();
        let clean_rate = rate(clean_records, total_records);
        let port_attempt_rate = rate(port_attempted_records, total_records);
        let telemetry = memory_submission_dashboard_telemetry(
            total_records,
            clean_records,
            repair_first_records,
            submitted_notes,
            failed_notes,
            blocked_reasons,
            attempted_notes,
            quality_reviewed_notes,
            quality_admitted_notes,
            quality_rejected_notes,
            port_attempted_records,
            no_note_records,
            clean_rate,
            port_attempt_rate,
        );

        Self {
            total_records,
            clean_records,
            repair_first_records,
            submitted_notes,
            failed_notes,
            blocked_reasons,
            attempted_notes,
            quality_reviewed_notes,
            quality_admitted_notes,
            quality_rejected_notes,
            port_attempted_records,
            no_note_records,
            clean_rate,
            port_attempt_rate,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn health(&self, policy: MemorySubmissionHealthPolicy) -> MemorySubmissionHealth {
        MemorySubmissionHealth::from_dashboard(self.clone(), policy)
    }
}

impl MemorySubmissionHealth {
    pub fn from_dashboard(
        dashboard: MemorySubmissionDashboard,
        policy: MemorySubmissionHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("memory_submission_history_empty".to_owned());
        } else if dashboard.clean_rate < policy.minimum_clean_rate {
            watch_reasons.push(format!(
                "memory_submission_clean_rate={:.3}<{}",
                dashboard.clean_rate, policy.minimum_clean_rate
            ));
        }

        if !dashboard.is_empty() && dashboard.port_attempt_rate < policy.minimum_port_attempt_rate {
            watch_reasons.push(format!(
                "memory_submission_port_attempt_rate={:.3}<{}",
                dashboard.port_attempt_rate, policy.minimum_port_attempt_rate
            ));
        }

        if dashboard.failed_notes > policy.maximum_failed_notes {
            repair_reasons.push(format!(
                "memory_submission_failed_notes={}>{}",
                dashboard.failed_notes, policy.maximum_failed_notes
            ));
        }

        if dashboard.blocked_reasons > policy.maximum_blocked_reasons {
            repair_reasons.push(format!(
                "memory_submission_blocked_reasons={}>{}",
                dashboard.blocked_reasons, policy.maximum_blocked_reasons
            ));
        }

        if dashboard.no_note_records > policy.maximum_no_note_records {
            watch_reasons.push(format!(
                "memory_submission_no_note_records={}>{}",
                dashboard.no_note_records, policy.maximum_no_note_records
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (MemorySubmissionHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (MemorySubmissionHealthStatus::Watch, watch_reasons)
        } else {
            (MemorySubmissionHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == MemorySubmissionHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != MemorySubmissionHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == MemorySubmissionHealthStatus::Repair
    }
}

impl MemorySubmissionSummaryHistoryRecord {
    pub fn records(&self) -> usize {
        self.history.len()
    }

    pub fn allows_service_advance(&self) -> bool {
        self.health.allows_service_advance()
    }

    pub fn requires_repair_first(&self) -> bool {
        self.health.requires_repair_first()
    }
}

impl MemorySubmissionSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: MemorySubmissionSummaryHistory,
        summary: MemorySubmissionSummary,
        policy: MemorySubmissionHealthPolicy,
    ) -> MemorySubmissionSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = memory_submission_history_record_telemetry(&dashboard, &health);

        MemorySubmissionSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_report_with_health(
        &self,
        history: MemorySubmissionSummaryHistory,
        report: &MemorySubmissionReport,
        policy: MemorySubmissionHealthPolicy,
    ) -> MemorySubmissionSummaryHistoryRecord {
        self.record_summary_with_health(history, report.summary(), policy)
    }
}

impl MemoryPromotionGate {
    pub fn new() -> Self {
        Self
    }

    pub fn gate(
        &self,
        candidate_notes: &[MemoryNote],
        reflection_gate: &ReflectionLoopHistoryGateDecision,
        aggregation_conflict_gate: &AggregationConflictReviewTrendGateDecision,
        memory_health: &MemorySubmissionHealth,
    ) -> MemoryPromotionGateDecision {
        let mut reasons = Vec::new();
        let note_quality = MemoryNoteQualityReport::from_notes(candidate_notes);

        if candidate_notes.is_empty() {
            reasons.push("memory_promotion_no_candidate_notes".to_owned());
        }
        if !candidate_notes.is_empty() && note_quality.admitted_notes == 0 {
            reasons.push("memory_promotion_no_admitted_candidate_notes".to_owned());
            extend_memory_ordered_unique(
                &mut reasons,
                note_quality
                    .rejection_reasons()
                    .into_iter()
                    .map(|reason| format!("memory_promotion_note_quality:{reason}"))
                    .collect(),
            );
        }

        if !reflection_gate.is_memory_promotable() {
            extend_memory_ordered_unique(
                &mut reasons,
                prefixed_or_default(
                    "memory_promotion_reflection",
                    &reflection_gate.reasons,
                    "memory_promotion_reflection_not_promotable",
                ),
            );
        }

        if !aggregation_conflict_gate.is_side_effect_safe() {
            extend_memory_ordered_unique(
                &mut reasons,
                prefixed_or_default(
                    "memory_promotion_aggregation_conflict",
                    &aggregation_conflict_gate.reasons,
                    "memory_promotion_aggregation_conflict_side_effect_closed",
                ),
            );
        }

        if !memory_health.is_stable() {
            extend_memory_ordered_unique(
                &mut reasons,
                prefixed_or_default(
                    "memory_promotion_submission_history",
                    &memory_health.reasons,
                    "memory_promotion_submission_history_not_stable",
                ),
            );
        }

        let requires_repair_first = reflection_gate.requires_repair_first
            || aggregation_conflict_gate.requires_repair_first
            || memory_health.requires_repair_first();
        let can_promote_memory_note = note_quality.admitted_notes > 0
            && reflection_gate.is_memory_promotable()
            && aggregation_conflict_gate.is_side_effect_safe()
            && memory_health.is_stable()
            && !requires_repair_first;
        let can_submit_memory = can_promote_memory_note;
        let mut repair_tasks = Vec::new();
        if reflection_gate.requires_repair_first {
            repair_tasks.extend(reflection_gate.repair_tasks.clone());
        }
        if aggregation_conflict_gate.requires_repair_first {
            repair_tasks.extend(aggregation_conflict_gate.repair_tasks.clone());
        }
        repair_tasks.extend(memory_promotion_gate_repair_tasks(
            memory_health.requires_repair_first(),
            &reasons,
        ));
        let telemetry = memory_promotion_gate_telemetry(
            candidate_notes.len(),
            note_quality.admitted_notes,
            note_quality.rejected_notes,
            can_promote_memory_note,
            can_submit_memory,
            requires_repair_first,
            repair_tasks.len(),
            reasons.len(),
            memory_health.status,
        );

        MemoryPromotionGateDecision {
            candidate_notes: candidate_notes.len(),
            admitted_candidate_notes: note_quality.admitted_notes,
            rejected_candidate_notes: note_quality.rejected_notes,
            note_quality,
            reflection_gate: reflection_gate.clone(),
            aggregation_conflict_gate: aggregation_conflict_gate.clone(),
            memory_health: memory_health.clone(),
            can_promote_memory_note,
            can_submit_memory,
            requires_repair_first,
            repair_tasks,
            reasons,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemorySubmissionFailure {
    pub note: MemoryNote,
    pub reason: String,
}

#[derive(Debug, Clone, Default)]
pub struct MemoryHandoffSubmitter;

impl MemoryHandoffSubmitter {
    pub fn new() -> Self {
        Self
    }

    pub fn submit<P>(&self, handoff: &AgentCycleHandoff, memory: &mut P) -> MemorySubmissionReport
    where
        P: MemoryPort,
        P::Error: ToString,
    {
        if !handoff.blocked_reasons.is_empty() {
            return MemorySubmissionReport {
                submitted: Vec::new(),
                failures: Vec::new(),
                blocked_reasons: handoff.blocked_reasons.clone(),
                note_quality: None,
            };
        }

        let note_quality = MemoryNoteQualityReport::from_notes(&handoff.memory_notes);
        if !handoff.memory_notes.is_empty() && note_quality.admitted_notes == 0 {
            let mut blocked_reasons = vec!["memory_submission_no_admitted_notes".to_owned()];
            blocked_reasons.extend(
                note_quality
                    .rejection_reasons()
                    .into_iter()
                    .map(|reason| format!("memory_submission_note_quality:{reason}")),
            );
            return MemorySubmissionReport {
                submitted: Vec::new(),
                failures: Vec::new(),
                blocked_reasons,
                note_quality: Some(note_quality),
            };
        }

        let mut submitted = Vec::new();
        let mut failures = Vec::new();
        for index in note_quality.admitted_indexes() {
            let note = &handoff.memory_notes[index];
            match memory.propose_note(note.clone()) {
                Ok(()) => submitted.push(note.clone()),
                Err(error) => failures.push(MemorySubmissionFailure {
                    note: note.clone(),
                    reason: error.to_string(),
                }),
            }
        }

        MemorySubmissionReport {
            submitted,
            failures,
            blocked_reasons: Vec::new(),
            note_quality: Some(note_quality),
        }
    }
}

fn failed_memory_recall_context(
    task: &AgentTask,
    query: String,
    requested_limit: usize,
    failure: String,
) -> MemoryRecallContext {
    let telemetry = memory_recall_context_telemetry(&task.id, 0, 0, 0, true);
    MemoryRecallContext {
        task_id: task.id.clone(),
        query,
        requested_limit,
        returned_records: 0,
        read_only: true,
        decisions: Vec::new(),
        failure: Some(failure),
        telemetry,
    }
}

fn memory_recall_query(task: &AgentTask) -> String {
    format!(
        "role:{} lane:{} objective:{}",
        task.role.as_str(),
        task.lane,
        task.objective
    )
}

fn normalized_source(source: &str) -> String {
    let source = source.trim();
    if source.is_empty() {
        "unknown".to_owned()
    } else {
        source.to_owned()
    }
}

fn compact_summary(summary: &str, max_summary_chars: usize) -> String {
    let max_summary_chars = max_summary_chars.max(1);
    let mut out = String::new();
    let mut previous_space = false;
    for ch in summary.trim().chars().take(max_summary_chars) {
        if ch.is_whitespace() {
            if !previous_space {
                out.push(' ');
                previous_space = true;
            }
        } else {
            out.push(ch);
            previous_space = false;
        }
    }
    out.trim().to_owned()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecallOutcomeTaskStatus {
    Accepted,
    Rejected,
    Failed,
}

fn recall_outcome_task_status(
    task_id: &str,
    execution: &AgentWaveExecution,
) -> Option<RecallOutcomeTaskStatus> {
    if execution
        .failures
        .iter()
        .any(|failure| failure.task_id == task_id)
    {
        return Some(RecallOutcomeTaskStatus::Failed);
    }

    let mut has_accepted_result = false;
    let mut has_rejected_result = false;
    for result in execution
        .results
        .iter()
        .filter(|result| result.task_id == task_id)
    {
        if result.accepted {
            has_accepted_result = true;
        } else {
            has_rejected_result = true;
        }
    }

    if has_rejected_result {
        Some(RecallOutcomeTaskStatus::Rejected)
    } else if has_accepted_result {
        Some(RecallOutcomeTaskStatus::Accepted)
    } else {
        None
    }
}

fn attribution_amount(amount: f32) -> f32 {
    if amount.is_finite() {
        amount.clamp(0.01, 1.0)
    } else {
        0.01
    }
}

fn memory_recall_context_telemetry(
    task_id: &str,
    returned: usize,
    admitted: usize,
    rejected: usize,
    failed: bool,
) -> Vec<String> {
    vec![
        "agent_memory_recall_context=true".to_owned(),
        format!("agent_memory_recall_context_task={task_id}"),
        format!("agent_memory_recall_context_returned={returned}"),
        format!("agent_memory_recall_context_admitted={admitted}"),
        format!("agent_memory_recall_context_rejected={rejected}"),
        format!("agent_memory_recall_context_failed={failed}"),
        "agent_memory_recall_context_read_only=true".to_owned(),
    ]
}

fn recall_outcome_attribution_telemetry(
    attribution_count: usize,
    reinforced_count: usize,
    penalized_count: usize,
    skipped_rejected_recall_count: usize,
    skipped_missing_outcome_task_count: usize,
) -> Vec<String> {
    vec![
        "agent_recall_outcome_attribution=true".to_owned(),
        "agent_recall_outcome_attribution_read_only=true".to_owned(),
        "agent_recall_outcome_attribution_memory_store_write_allowed=false".to_owned(),
        format!("agent_recall_outcome_attribution_updates={attribution_count}"),
        format!("agent_recall_outcome_attribution_reinforce={reinforced_count}"),
        format!("agent_recall_outcome_attribution_penalize={penalized_count}"),
        format!(
            "agent_recall_outcome_attribution_skipped_rejected_recall={skipped_rejected_recall_count}"
        ),
        format!(
            "agent_recall_outcome_attribution_skipped_missing_outcome_tasks={skipped_missing_outcome_task_count}"
        ),
    ]
}

fn memory_recall_dry_run_evidence_telemetry(
    task: &AgentTask,
    evidence: &MemoryRecallDryRunEvidence,
    admitted: bool,
    rejected: bool,
) -> Vec<String> {
    vec![
        "agent_memory_recall_dry_run_evidence=true".to_owned(),
        format!("agent_memory_recall_dry_run_evidence_task={}", task.id),
        format!(
            "agent_memory_recall_dry_run_evidence_source={}",
            normalized_source(&evidence.source)
        ),
        format!(
            "agent_memory_recall_dry_run_evidence_read_only={}",
            evidence.read_only
        ),
        format!(
            "agent_memory_recall_dry_run_evidence_safe={}",
            evidence.safe_for_recall_sidecar()
        ),
        format!("agent_memory_recall_dry_run_evidence_admitted={admitted}"),
        format!("agent_memory_recall_dry_run_evidence_rejected={rejected}"),
        format!(
            "agent_memory_recall_dry_run_evidence_candidates={}",
            evidence.candidate_count
        ),
        format!(
            "agent_memory_recall_dry_run_evidence_context_accepted={}",
            evidence.accepted_context_count
        ),
        format!(
            "agent_memory_recall_dry_run_evidence_context_rejected={}",
            evidence.rejected_context_count
        ),
        format!(
            "agent_memory_recall_dry_run_evidence_kv_requested={}",
            evidence.requested_kv_count
        ),
        format!(
            "agent_memory_recall_dry_run_evidence_kv_promote={}",
            evidence.kv_promote_count
        ),
        format!(
            "agent_memory_recall_dry_run_evidence_kv_missing={}",
            evidence.kv_missing_count
        ),
        format!(
            "agent_memory_recall_dry_run_evidence_memory_store_write_allowed={}",
            evidence.memory_store_write_allowed
        ),
        format!(
            "agent_memory_recall_dry_run_evidence_kv_prefetch_apply_allowed={}",
            evidence.kv_prefetch_apply_allowed
        ),
    ]
}

fn memory_reuse_execution_preflight_telemetry(
    task_count: usize,
    recall_context_count: usize,
    dry_run_evidence_count: usize,
    memory_reuse_ready: bool,
    can_enter_execution: bool,
    requires_repair_first: bool,
    accepted_recall_count: usize,
    rejected_recall_count: usize,
    kv_requested_count: usize,
    kv_promote_count: usize,
    kv_missing_count: usize,
) -> Vec<String> {
    vec![
        "agent_memory_reuse_execution_preflight=true".to_owned(),
        "agent_memory_reuse_execution_preflight_read_only=true".to_owned(),
        "agent_memory_reuse_execution_preflight_prompt_injection_allowed=false".to_owned(),
        "agent_memory_reuse_execution_preflight_engine_port_touched=false".to_owned(),
        format!("agent_memory_reuse_execution_preflight_tasks={task_count}"),
        format!("agent_memory_reuse_execution_preflight_recall_contexts={recall_context_count}"),
        format!("agent_memory_reuse_execution_preflight_dry_run_evidence={dry_run_evidence_count}"),
        format!("agent_memory_reuse_execution_preflight_ready={memory_reuse_ready}"),
        format!("agent_memory_reuse_execution_preflight_can_enter_execution={can_enter_execution}"),
        format!(
            "agent_memory_reuse_execution_preflight_requires_repair_first={requires_repair_first}"
        ),
        format!("agent_memory_reuse_execution_preflight_accepted_recall={accepted_recall_count}"),
        format!("agent_memory_reuse_execution_preflight_rejected_recall={rejected_recall_count}"),
        format!("agent_memory_reuse_execution_preflight_kv_requested={kv_requested_count}"),
        format!("agent_memory_reuse_execution_preflight_kv_promote={kv_promote_count}"),
        format!("agent_memory_reuse_execution_preflight_kv_missing={kv_missing_count}"),
    ]
}

fn agent_wave_memory_recall_telemetry(contexts: &[MemoryRecallContext]) -> Vec<String> {
    let plan = AgentWaveMemoryRecallPlan {
        contexts: contexts.to_vec(),
        telemetry: Vec::new(),
    };
    vec![
        "agent_wave_memory_recall=true".to_owned(),
        format!("agent_wave_memory_recall_tasks={}", plan.task_count()),
        format!(
            "agent_wave_memory_recall_admitted={}",
            plan.accepted_count()
        ),
        format!(
            "agent_wave_memory_recall_rejected={}",
            plan.rejected_count()
        ),
        format!("agent_wave_memory_recall_failed={}", plan.failed_count()),
        format!("agent_wave_memory_recall_read_only={}", plan.read_only()),
    ]
}

fn join_codes(codes: Vec<String>) -> String {
    if codes.is_empty() {
        "none".to_owned()
    } else {
        codes.join("|")
    }
}

fn normalized_note_field(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn hex_id(id: &str) -> String {
    id.as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

fn memory_note_quality_report_telemetry(
    candidate_notes: usize,
    admitted_notes: usize,
    rejected_notes: usize,
) -> Vec<String> {
    vec![
        "agent_memory_note_quality=true".to_owned(),
        format!("agent_memory_note_quality_candidate_notes={candidate_notes}"),
        format!("agent_memory_note_quality_admitted_notes={admitted_notes}"),
        format!("agent_memory_note_quality_rejected_notes={rejected_notes}"),
    ]
}

fn memory_submission_summary_telemetry(
    submitted_notes: usize,
    failed_notes: usize,
    blocked_reasons: usize,
    attempted_notes: usize,
    quality_reviewed_notes: usize,
    quality_admitted_notes: usize,
    quality_rejected_notes: usize,
    clean: bool,
    port_attempted: bool,
) -> Vec<String> {
    vec![
        "agent_memory_submission_summary=true".to_owned(),
        format!("agent_memory_submission_summary_submitted_notes={submitted_notes}"),
        format!("agent_memory_submission_summary_failed_notes={failed_notes}"),
        format!("agent_memory_submission_summary_blocked_reasons={blocked_reasons}"),
        format!("agent_memory_submission_summary_attempted_notes={attempted_notes}"),
        format!("agent_memory_submission_summary_quality_reviewed_notes={quality_reviewed_notes}"),
        format!("agent_memory_submission_summary_quality_admitted_notes={quality_admitted_notes}"),
        format!("agent_memory_submission_summary_quality_rejected_notes={quality_rejected_notes}"),
        format!("agent_memory_submission_summary_clean={clean}"),
        format!("agent_memory_submission_summary_port_attempted={port_attempted}"),
    ]
}

fn memory_submission_gate_telemetry(
    can_continue_loop: bool,
    can_commit_submitted_notes: bool,
    requires_repair_first: bool,
    reasons: usize,
    summary: &MemorySubmissionSummary,
) -> Vec<String> {
    vec![
        "agent_memory_submission_gate=true".to_owned(),
        format!("agent_memory_submission_gate_continue={can_continue_loop}"),
        format!("agent_memory_submission_gate_commit_notes={can_commit_submitted_notes}"),
        format!("agent_memory_submission_gate_repair_first={requires_repair_first}"),
        format!("agent_memory_submission_gate_reasons={reasons}"),
        format!(
            "agent_memory_submission_gate_submitted_notes={}",
            summary.submitted_notes
        ),
        format!(
            "agent_memory_submission_gate_failed_notes={}",
            summary.failed_notes
        ),
        format!(
            "agent_memory_submission_gate_blocked_reasons={}",
            summary.blocked_reasons
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn memory_submission_dashboard_telemetry(
    total_records: usize,
    clean_records: usize,
    repair_first_records: usize,
    submitted_notes: usize,
    failed_notes: usize,
    blocked_reasons: usize,
    attempted_notes: usize,
    quality_reviewed_notes: usize,
    quality_admitted_notes: usize,
    quality_rejected_notes: usize,
    port_attempted_records: usize,
    no_note_records: usize,
    clean_rate: f32,
    port_attempt_rate: f32,
) -> Vec<String> {
    vec![
        "agent_memory_submission_dashboard=true".to_owned(),
        format!("agent_memory_submission_dashboard_records={total_records}"),
        format!("agent_memory_submission_dashboard_clean={clean_records}"),
        format!("agent_memory_submission_dashboard_repair_first={repair_first_records}"),
        format!("agent_memory_submission_dashboard_submitted_notes={submitted_notes}"),
        format!("agent_memory_submission_dashboard_failed_notes={failed_notes}"),
        format!("agent_memory_submission_dashboard_blocked_reasons={blocked_reasons}"),
        format!("agent_memory_submission_dashboard_attempted_notes={attempted_notes}"),
        format!(
            "agent_memory_submission_dashboard_quality_reviewed_notes={quality_reviewed_notes}"
        ),
        format!(
            "agent_memory_submission_dashboard_quality_admitted_notes={quality_admitted_notes}"
        ),
        format!(
            "agent_memory_submission_dashboard_quality_rejected_notes={quality_rejected_notes}"
        ),
        format!("agent_memory_submission_dashboard_port_attempted={port_attempted_records}"),
        format!("agent_memory_submission_dashboard_no_note={no_note_records}"),
        format!("agent_memory_submission_dashboard_clean_rate={clean_rate:.3}"),
        format!("agent_memory_submission_dashboard_port_attempt_rate={port_attempt_rate:.3}"),
    ]
}

fn memory_submission_history_record_telemetry(
    dashboard: &MemorySubmissionDashboard,
    health: &MemorySubmissionHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_memory_submission_history_record=true".to_owned(),
        format!(
            "agent_memory_submission_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_memory_submission_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_memory_submission_history_record_clean_rate={:.3}",
            dashboard.clean_rate
        ),
        format!(
            "agent_memory_submission_history_record_failed_notes={}",
            dashboard.failed_notes
        ),
        format!(
            "agent_memory_submission_history_record_blocked_reasons={}",
            dashboard.blocked_reasons
        ),
        format!(
            "agent_memory_submission_history_record_quality_rejected_notes={}",
            dashboard.quality_rejected_notes
        ),
    ];
    telemetry.extend(
        health
            .reasons
            .iter()
            .map(|reason| format!("agent_memory_submission_history_record_reason={reason}")),
    );
    telemetry
}

fn prefixed_or_default(prefix: &str, reasons: &[String], default_reason: &str) -> Vec<String> {
    if reasons.is_empty() {
        return vec![default_reason.to_owned()];
    }

    reasons
        .iter()
        .map(|reason| format!("{prefix}:{reason}"))
        .collect()
}

fn extend_memory_ordered_unique(target: &mut Vec<String>, items: Vec<String>) {
    for item in items {
        if !target.contains(&item) {
            target.push(item);
        }
    }
}

fn memory_promotion_gate_repair_tasks(
    requires_repair_first: bool,
    reasons: &[String],
) -> Vec<AgentTask> {
    if !requires_repair_first {
        return Vec::new();
    }

    reasons
        .iter()
        .enumerate()
        .map(|(index, reason)| {
            AgentTask::new(
                format!("memory-promotion-repair-{index}"),
                AgentRole::MemoryCurator,
                format!("repair memory promotion gate: {reason}"),
                AgentBudget::new(16, 1, 1),
            )
            .with_lane("memory-promotion-repair")
            .with_priority(1)
        })
        .collect()
}

fn memory_promotion_gate_telemetry(
    candidate_notes: usize,
    admitted_candidate_notes: usize,
    rejected_candidate_notes: usize,
    can_promote_memory_note: bool,
    can_submit_memory: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    reasons: usize,
    memory_health_status: MemorySubmissionHealthStatus,
) -> Vec<String> {
    vec![
        "agent_memory_promotion_gate=true".to_owned(),
        format!("agent_memory_promotion_gate_candidate_notes={candidate_notes}"),
        format!("agent_memory_promotion_gate_admitted_candidate_notes={admitted_candidate_notes}"),
        format!("agent_memory_promotion_gate_rejected_candidate_notes={rejected_candidate_notes}"),
        format!("agent_memory_promotion_gate_promote={can_promote_memory_note}"),
        format!("agent_memory_promotion_gate_submit={can_submit_memory}"),
        format!("agent_memory_promotion_gate_requires_repair_first={requires_repair_first}"),
        format!("agent_memory_promotion_gate_repair_tasks={repair_tasks}"),
        format!("agent_memory_promotion_gate_reasons={reasons}"),
        format!(
            "agent_memory_promotion_gate_memory_health={}",
            memory_health_status.as_str()
        ),
    ]
}

fn rate(numerator: usize, denominator: usize) -> f32 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f32 / denominator as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aggregate::{
        AggregationConflictReviewHealthPolicy, AggregationConflictReviewSummaryHistory,
        AggregationConflictReviewSummaryHistoryRecorder, AggregationConflictReviewTrendGate,
        AggregationConflictReviewer, AggregationHealthPolicy, AggregationSummaryHistory,
    };
    use crate::conflict::{ConflictReportHealthPolicy, ConflictReportSummaryHistory};
    use crate::execute::{AgentExecutionFailure, AgentWaveExecution};
    use crate::message::{AgentMessage, AgentMessageKind};
    use crate::reflection::{
        ReflectionLoop, ReflectionLoopHealthPolicy, ReflectionLoopSummaryHistory,
        ReflectionLoopSummaryHistoryRecorder, ReflectionStage,
    };
    use crate::schedule::AgentExecutionWave;
    use crate::task::{AgentRole, AgentTask, TaskAssignment, TaskDispatchPlan};

    #[derive(Debug, Default)]
    struct FakeMemoryPort {
        fail_topic: Option<String>,
        fail_recall: bool,
        records: Vec<MemoryRecord>,
        recall_queries: std::cell::RefCell<Vec<(String, usize)>>,
        submitted: Vec<MemoryNote>,
    }

    impl MemoryPort for FakeMemoryPort {
        type Error = String;

        fn recall(
            &self,
            query: &str,
            limit: usize,
        ) -> Result<Vec<crate::ports::MemoryRecord>, Self::Error> {
            self.recall_queries
                .borrow_mut()
                .push((query.to_owned(), limit));
            if self.fail_recall {
                return Err("recall backend unavailable".to_owned());
            }
            Ok(self.records.clone())
        }

        fn propose_note(&mut self, note: MemoryNote) -> Result<(), Self::Error> {
            if self.fail_topic.as_deref() == Some(note.topic.as_str()) {
                return Err(format!("memory rejected {}", note.topic));
            }
            self.submitted.push(note);
            Ok(())
        }
    }

    #[test]
    fn submitter_writes_clean_handoff_notes_through_memory_port() {
        let handoff = AgentCycleHandoff {
            memory_notes: vec![MemoryNote::new("agent_cycle", "remember clean loop")],
            follow_up_tasks: Vec::new(),
            blocked_reasons: Vec::new(),
        };
        let mut memory = FakeMemoryPort::default();

        let report = MemoryHandoffSubmitter::new().submit(&handoff, &mut memory);

        assert!(report.is_clean());
        assert_eq!(report.submitted.len(), 1);
        assert_eq!(memory.submitted.len(), 1);
        assert_eq!(memory.submitted[0].content, "remember clean loop");

        let summary = report.summary();
        assert_eq!(summary.submitted_notes, 1);
        assert_eq!(summary.failed_notes, 0);
        assert_eq!(summary.attempted_notes, 1);
        assert!(summary.clean);
        assert!(summary.port_attempted);

        let gate = report.gate();
        assert!(gate.can_continue_loop);
        assert!(gate.can_commit_submitted_notes);
        assert!(!gate.requires_repair_first);
        assert!(gate.reasons.is_empty());
    }

    #[test]
    fn submitter_filters_invalid_handoff_notes_before_memory_port() {
        let handoff = AgentCycleHandoff {
            memory_notes: vec![
                MemoryNote::new("agent_cycle", "remember clean loop"),
                MemoryNote::new("", "missing topic"),
                MemoryNote::new("agent_cycle", "remember   clean   loop"),
                MemoryNote::new("agent_cycle", "   "),
            ],
            follow_up_tasks: Vec::new(),
            blocked_reasons: Vec::new(),
        };
        let mut memory = FakeMemoryPort::default();

        let report = MemoryHandoffSubmitter::new().submit(&handoff, &mut memory);

        assert!(report.is_clean());
        assert_eq!(report.submitted.len(), 1);
        assert_eq!(report.submitted[0].content, "remember clean loop");
        assert_eq!(memory.submitted.len(), 1);
        assert_eq!(memory.submitted[0].topic, "agent_cycle");
        assert!(report.failures.is_empty());
        assert!(report.blocked_reasons.is_empty());

        let summary = report.summary();
        assert_eq!(summary.submitted_notes, 1);
        assert_eq!(summary.failed_notes, 0);
        assert_eq!(summary.attempted_notes, 1);
        assert_eq!(summary.quality_reviewed_notes, 4);
        assert_eq!(summary.quality_admitted_notes, 1);
        assert_eq!(summary.quality_rejected_notes, 3);
        assert!(summary.clean);
        assert!(
            summary
                .telemetry
                .iter()
                .any(|line| line == "agent_memory_submission_summary_quality_rejected_notes=3")
        );

        let gate = report.gate();
        assert!(gate.can_commit_submitted_notes);
        assert!(gate.reasons.is_empty());

        let record = MemorySubmissionSummaryHistoryRecorder::new().record_report_with_health(
            MemorySubmissionSummaryHistory::new(),
            &report,
            MemorySubmissionHealthPolicy::default(),
        );
        assert_eq!(record.dashboard.quality_reviewed_notes, 4);
        assert_eq!(record.dashboard.quality_admitted_notes, 1);
        assert_eq!(record.dashboard.quality_rejected_notes, 3);
        assert_eq!(record.health.status, MemorySubmissionHealthStatus::Stable);
        assert!(
            record
                .dashboard
                .telemetry
                .iter()
                .any(|line| line == "agent_memory_submission_dashboard_quality_rejected_notes=3")
        );
    }

    #[test]
    fn submitter_blocks_all_invalid_handoff_notes_without_memory_port_call() {
        let handoff = AgentCycleHandoff {
            memory_notes: vec![
                MemoryNote::new("", "missing topic"),
                MemoryNote::new("agent_cycle", "   "),
            ],
            follow_up_tasks: Vec::new(),
            blocked_reasons: Vec::new(),
        };
        let mut memory = FakeMemoryPort::default();

        let report = MemoryHandoffSubmitter::new().submit(&handoff, &mut memory);

        assert!(!report.is_clean());
        assert!(report.submitted.is_empty());
        assert!(report.failures.is_empty());
        assert!(memory.submitted.is_empty());
        assert_eq!(
            report.blocked_reasons,
            vec![
                "memory_submission_no_admitted_notes",
                "memory_submission_note_quality:memory_note_quality_empty_topic index=0",
                "memory_submission_note_quality:memory_note_quality_empty_content index=1",
            ]
        );

        let summary = report.summary();
        assert_eq!(summary.submitted_notes, 0);
        assert_eq!(summary.failed_notes, 0);
        assert_eq!(summary.blocked_reasons, 3);
        assert_eq!(summary.attempted_notes, 0);
        assert_eq!(summary.quality_reviewed_notes, 2);
        assert_eq!(summary.quality_admitted_notes, 0);
        assert_eq!(summary.quality_rejected_notes, 2);
        assert!(!summary.clean);
        assert!(!summary.port_attempted);

        let gate = report.gate();
        assert!(!gate.can_continue_loop);
        assert!(!gate.can_commit_submitted_notes);
        assert!(gate.requires_repair_first);
        assert_eq!(
            gate.reasons,
            vec![
                "memory_handoff_blocked reason=memory_submission_no_admitted_notes",
                "memory_handoff_blocked reason=memory_submission_note_quality:memory_note_quality_empty_topic index=0",
                "memory_handoff_blocked reason=memory_submission_note_quality:memory_note_quality_empty_content index=1",
            ]
        );
    }

    #[test]
    fn submitter_does_not_call_memory_port_when_handoff_is_blocked() {
        let handoff = AgentCycleHandoff {
            memory_notes: vec![MemoryNote::new("agent_cycle", "remember blocked loop")],
            follow_up_tasks: vec![AgentTask::new(
                "repair",
                AgentRole::Reviewer,
                "repair loop",
                crate::budget::AgentBudget::new(1, 1, 1),
            )],
            blocked_reasons: vec!["unresolved_conflicts=1".to_owned()],
        };
        let mut memory = FakeMemoryPort::default();

        let report = MemoryHandoffSubmitter::new().submit(&handoff, &mut memory);

        assert!(!report.is_clean());
        assert!(report.submitted.is_empty());
        assert!(memory.submitted.is_empty());
        assert_eq!(report.blocked_reasons, vec!["unresolved_conflicts=1"]);

        let summary = report.summary();
        assert_eq!(summary.submitted_notes, 0);
        assert_eq!(summary.failed_notes, 0);
        assert_eq!(summary.blocked_reasons, 1);
        assert_eq!(summary.attempted_notes, 0);
        assert!(!summary.clean);
        assert!(!summary.port_attempted);

        let gate = report.gate();
        assert!(!gate.can_continue_loop);
        assert!(!gate.can_commit_submitted_notes);
        assert!(gate.requires_repair_first);
        assert_eq!(
            gate.reasons,
            vec!["memory_handoff_blocked reason=unresolved_conflicts=1"]
        );
    }

    #[test]
    fn submitter_records_memory_port_failures_as_data() {
        let handoff = AgentCycleHandoff {
            memory_notes: vec![MemoryNote::new("agent_cycle", "remember clean loop")],
            follow_up_tasks: Vec::new(),
            blocked_reasons: Vec::new(),
        };
        let mut memory = FakeMemoryPort {
            fail_topic: Some("agent_cycle".to_owned()),
            ..FakeMemoryPort::default()
        };

        let report = MemoryHandoffSubmitter::new().submit(&handoff, &mut memory);

        assert!(!report.is_clean());
        assert!(report.submitted.is_empty());
        assert_eq!(report.failures.len(), 1);
        assert_eq!(report.failures[0].reason, "memory rejected agent_cycle");

        let summary = report.summary();
        assert_eq!(summary.submitted_notes, 0);
        assert_eq!(summary.failed_notes, 1);
        assert_eq!(summary.blocked_reasons, 0);
        assert_eq!(summary.attempted_notes, 1);
        assert!(!summary.clean);
        assert!(summary.port_attempted);

        let gate = report.gate();
        assert!(!gate.can_continue_loop);
        assert!(!gate.can_commit_submitted_notes);
        assert!(gate.requires_repair_first);
        assert_eq!(
            gate.reasons,
            vec!["memory_submission_failed topic=agent_cycle reason=memory rejected agent_cycle"]
        );
    }

    #[test]
    fn recall_context_planner_builds_read_only_task_sidecar() {
        let task = AgentTask::new(
            "runtime-coder",
            AgentRole::Coder,
            "repair runtime memory reuse",
            AgentBudget::new(8, 1, 1),
        )
        .with_lane("runtime");
        let memory = FakeMemoryPort {
            records: vec![
                MemoryRecord::new(
                    "lesson-1",
                    "Use context gate before runtime KV prefetch so dirty records stay out",
                    "long_term",
                ),
                MemoryRecord::new("lesson-1", "duplicate should be rejected", "long_term"),
                MemoryRecord::new("empty-summary", "   ", "long_term"),
                MemoryRecord::new("lesson-2", "second accepted memory", ""),
                MemoryRecord::new("overflow", "budget rejects this", "long_term"),
            ],
            ..FakeMemoryPort::default()
        };
        let planner = MemoryRecallContextPlanner::new().with_policy(MemoryRecallPolicy {
            limit_per_task: 5,
            max_context_records_per_task: 2,
            max_summary_chars: 32,
        });

        let context = planner.plan_for_task(&task, &memory);

        assert!(context.read_only);
        assert_eq!(context.task_id, "runtime-coder");
        assert!(context.query.contains("role:coder"));
        assert!(context.query.contains("lane:runtime"));
        assert!(context.query.contains("repair runtime memory reuse"));
        assert_eq!(
            memory.recall_queries.borrow().as_slice(),
            &[(
                "role:coder lane:runtime objective:repair runtime memory reuse".to_owned(),
                5
            )]
        );
        assert_eq!(context.returned_records, 5);
        assert_eq!(context.accepted_count(), 2);
        assert_eq!(context.rejected_count(), 3);
        assert_eq!(
            context
                .accepted_items()
                .into_iter()
                .map(|item| (item.id.as_str(), item.source.as_str()))
                .collect::<Vec<_>>(),
            vec![("lesson-1", "long_term"), ("lesson-2", "unknown")]
        );
        assert!(context.context_lines()[0].contains("Use context gate before runtime"));
        assert!(context.decisions.iter().any(|decision| {
            decision.kind == MemoryRecallDecisionKind::RejectDuplicate
                && decision.reasons == vec!["duplicate_id"]
        }));
        assert!(context.decisions.iter().any(|decision| {
            decision.kind == MemoryRecallDecisionKind::RejectEmptySummary
                && decision.reasons == vec!["empty_summary"]
        }));
        assert!(context.decisions.iter().any(|decision| {
            decision.kind == MemoryRecallDecisionKind::RejectBudget
                && decision.reasons == vec!["max_context_records"]
        }));
        assert!(context.summary_line().contains("read_only=true"));
    }

    #[test]
    fn wave_recall_planner_builds_contexts_for_assigned_tasks_only() {
        let coder = AgentTask::new(
            "coder",
            AgentRole::Coder,
            "write memory reuse code",
            AgentBudget::new(4, 1, 1),
        );
        let reviewer = AgentTask::new(
            "reviewer",
            AgentRole::Reviewer,
            "review memory reuse code",
            AgentBudget::new(4, 1, 1),
        );
        let memory = FakeMemoryPort {
            records: vec![MemoryRecord::new(
                "shared",
                "shared lesson for the wave",
                "long_term",
            )],
            ..FakeMemoryPort::default()
        };
        let dispatch = AgentCycleDispatch {
            wave: AgentExecutionWave {
                wave: 0,
                task_ids: vec!["coder".to_owned(), "reviewer".to_owned()],
                parallel_count: 2,
            },
            dispatch: TaskDispatchPlan {
                assignments: vec![
                    TaskAssignment {
                        task_id: "coder".to_owned(),
                        role: AgentRole::Coder,
                        lane: "default".to_owned(),
                        budget_reserved: AgentBudget::new(4, 1, 1),
                    },
                    TaskAssignment {
                        task_id: "reviewer".to_owned(),
                        role: AgentRole::Reviewer,
                        lane: "default".to_owned(),
                        budget_reserved: AgentBudget::new(4, 1, 1),
                    },
                ],
                ..TaskDispatchPlan::default()
            },
            assigned_tasks: vec![coder, reviewer],
            blocked_task_ids: vec!["blocked".to_owned()],
            remaining_queue: crate::task::AgentTaskQueue::new(),
        };

        let plan = AgentWaveMemoryRecallPlanner::new().plan_dispatch(&dispatch, &memory);

        assert!(plan.read_only());
        assert_eq!(plan.task_count(), 2);
        assert_eq!(plan.accepted_count(), 2);
        assert_eq!(plan.rejected_count(), 0);
        assert_eq!(plan.failed_count(), 0);
        assert_eq!(memory.recall_queries.borrow().len(), 2);
        assert!(
            plan.telemetry
                .iter()
                .any(|line| line == "agent_wave_memory_recall_read_only=true")
        );
        assert_eq!(
            plan.summary_line(),
            "agent_wave_memory_recall read_only=true tasks=2 admitted=2 rejected=0 failed=0"
        );
    }

    #[test]
    fn recall_context_planner_reports_failures_without_writes() {
        let task = AgentTask::new(
            "tester",
            AgentRole::Tester,
            "validate memory recall",
            AgentBudget::new(4, 1, 1),
        );
        let memory = FakeMemoryPort {
            fail_recall: true,
            ..FakeMemoryPort::default()
        };

        let context = MemoryRecallContextPlanner::new().plan_for_task(&task, &memory);

        assert!(context.read_only);
        assert!(context.failed());
        assert_eq!(
            context.failure.as_deref(),
            Some("recall backend unavailable")
        );
        assert_eq!(context.accepted_count(), 0);
        assert_eq!(context.rejected_count(), 0);
        assert_eq!(memory.submitted.len(), 0);
        assert!(context.reason_codes().contains(&"recall_failed".to_owned()));
        assert!(context.summary_line().contains("failed=true"));
    }

    #[test]
    fn recall_context_planner_rejects_write_enabled_dry_run_sidecars() {
        let task = AgentTask::new(
            "memory-reuse",
            AgentRole::MemoryCurator,
            "inspect reuse dry run before admission",
            AgentBudget::new(4, 1, 1),
        );
        let evidence = MemoryRecallDryRunEvidence {
            source: "norion_memory_reuse_dry_run".to_owned(),
            read_only: true,
            candidate_count: 1,
            long_term_match_count: 1,
            context_decision_count: 1,
            accepted_context_count: 1,
            rejected_context_count: 0,
            used_tokens: 48,
            requested_kv_count: 1,
            kv_promote_count: 1,
            kv_missing_count: 0,
            kv_already_hot_count: 0,
            kv_duplicate_count: 0,
            kv_backend_available: true,
            memory_store_write_allowed: true,
            kv_prefetch_apply_allowed: false,
            reason_codes: vec!["read_only".to_owned(), "context_accepted".to_owned()],
            detail_codes: vec!["kv_prefetch:promote:636f6c64".to_owned()],
        };

        let context =
            MemoryRecallContextPlanner::new().plan_from_dry_run_evidence(&task, &evidence);

        assert!(!evidence.safe_for_recall_sidecar());
        assert!(context.read_only);
        assert_eq!(context.returned_records, 1);
        assert_eq!(context.accepted_count(), 0);
        assert_eq!(context.rejected_count(), 1);
        assert!(context.context_lines().is_empty());
        assert_eq!(
            context.decisions[0].kind,
            MemoryRecallDecisionKind::RejectUnsafeSidecar
        );
        assert!(
            context.decisions[0]
                .reasons
                .contains(&"memory_store_write_allowed".to_owned())
        );
        assert!(
            context
                .reason_codes()
                .contains(&"memory_store_write_allowed".to_owned())
        );
        assert!(
            context
                .telemetry
                .iter()
                .any(|line| { line == "agent_memory_recall_dry_run_evidence_safe=false" })
        );
    }

    #[test]
    fn recall_outcome_attribution_reinforces_accepted_recall_for_accepted_result() {
        let task = AgentTask::new(
            "runtime-coder",
            AgentRole::Coder,
            "reuse runtime memory",
            AgentBudget::new(8, 1, 1),
        );
        let recall_context = MemoryRecallContextPlanner::new()
            .with_policy(MemoryRecallPolicy {
                limit_per_task: 3,
                max_context_records_per_task: 1,
                max_summary_chars: 64,
            })
            .plan_from_records(
                &task,
                "role:coder lane:default objective:reuse runtime memory".to_owned(),
                vec![
                    MemoryRecord::new("lesson-1", "reuse this accepted lesson", "long_term"),
                    MemoryRecord::new("lesson-2", "budget should reject this", "long_term"),
                ],
            );
        let recall_plan = AgentWaveMemoryRecallPlan {
            contexts: vec![recall_context],
            telemetry: Vec::new(),
        };
        let execution = AgentWaveExecution {
            results: vec![crate::task::AgentResult::accepted(
                &task,
                "runtime reuse worked",
                Vec::new(),
                AgentBudget::new(1, 1, 1),
            )],
            failures: Vec::new(),
        };

        let report = AgentRecallOutcomeAttributionPlanner::new().plan(&recall_plan, &execution);

        assert!(report.read_only);
        assert!(!report.memory_store_write_allowed);
        assert!(report.has_updates());
        assert_eq!(report.reinforced_count, 1);
        assert_eq!(report.penalized_count, 0);
        assert_eq!(report.skipped_rejected_recall_count, 1);
        assert!(report.skipped_missing_outcome_task_ids.is_empty());
        assert_eq!(report.attributions.len(), 1);
        assert_eq!(report.attributions[0].task_id, "runtime-coder");
        assert_eq!(report.attributions[0].record_id, "lesson-1");
        assert_eq!(report.attributions[0].source, "long_term");
        assert_eq!(
            report.attributions[0].action,
            AgentRecallOutcomeAttributionAction::Reinforce
        );
        assert_eq!(report.attributions[0].amount, 0.24);
        assert!(
            report.attributions[0]
                .reason_codes
                .contains(&"result_accepted".to_owned())
        );
        assert!(
            report
                .telemetry
                .iter()
                .any(|line| line == "agent_recall_outcome_attribution_reinforce=1")
        );
        assert!(report.summary_line().contains("updates=1"));
    }

    #[test]
    fn recall_outcome_attribution_penalizes_failed_and_rejected_results_only() {
        let failed_task = AgentTask::new(
            "runtime-coder",
            AgentRole::Coder,
            "reuse runtime memory",
            AgentBudget::new(8, 1, 1),
        );
        let rejected_task = AgentTask::new(
            "runtime-reviewer",
            AgentRole::Reviewer,
            "review runtime memory",
            AgentBudget::new(8, 1, 1),
        );
        let unsafe_task = AgentTask::new(
            "unsafe-sidecar",
            AgentRole::MemoryCurator,
            "inspect unsafe dry run",
            AgentBudget::new(4, 1, 1),
        );
        let planner = MemoryRecallContextPlanner::new();
        let failed_context = planner.plan_from_records(
            &failed_task,
            "role:coder lane:default objective:reuse runtime memory".to_owned(),
            vec![MemoryRecord::new(
                "lesson-fail",
                "failed lesson",
                "long_term",
            )],
        );
        let rejected_context = planner.plan_from_records(
            &rejected_task,
            "role:reviewer lane:default objective:review runtime memory".to_owned(),
            vec![MemoryRecord::new(
                "lesson-reject",
                "rejected lesson",
                "long_term",
            )],
        );
        let unsafe_evidence = MemoryRecallDryRunEvidence {
            source: "unsafe_dry_run".to_owned(),
            read_only: true,
            candidate_count: 1,
            long_term_match_count: 1,
            context_decision_count: 1,
            accepted_context_count: 1,
            rejected_context_count: 0,
            used_tokens: 32,
            requested_kv_count: 1,
            kv_promote_count: 1,
            kv_missing_count: 0,
            kv_already_hot_count: 0,
            kv_duplicate_count: 0,
            kv_backend_available: true,
            memory_store_write_allowed: true,
            kv_prefetch_apply_allowed: false,
            reason_codes: vec!["read_only".to_owned()],
            detail_codes: Vec::new(),
        };
        let unsafe_context = planner.plan_from_dry_run_evidence(&unsafe_task, &unsafe_evidence);
        let recall_plan = AgentWaveMemoryRecallPlan {
            contexts: vec![failed_context, rejected_context, unsafe_context],
            telemetry: Vec::new(),
        };
        let execution = AgentWaveExecution {
            results: vec![
                crate::task::AgentResult::rejected(&rejected_task, "review rejected memory use"),
                crate::task::AgentResult::accepted(
                    &unsafe_task,
                    "unsafe sidecar task finished",
                    Vec::new(),
                    AgentBudget::new(1, 1, 1),
                ),
            ],
            failures: vec![AgentExecutionFailure {
                task_id: failed_task.id.clone(),
                role: failed_task.role.clone(),
                reason: "engine failed".to_owned(),
            }],
        };

        let report = AgentRecallOutcomeAttributionPlanner::new().plan(&recall_plan, &execution);

        assert_eq!(report.attributions.len(), 2);
        assert_eq!(report.reinforced_count, 0);
        assert_eq!(report.penalized_count, 2);
        assert_eq!(report.skipped_rejected_recall_count, 1);
        assert!(report.skipped_missing_outcome_task_ids.is_empty());
        assert!(report.attributions.iter().all(|attribution| {
            attribution.action == AgentRecallOutcomeAttributionAction::Penalize
        }));
        let failed = report
            .attributions
            .iter()
            .find(|attribution| attribution.record_id == "lesson-fail")
            .unwrap();
        assert_eq!(failed.amount, 0.32);
        assert!(failed.reason_codes.contains(&"execution_failed".to_owned()));
        let rejected = report
            .attributions
            .iter()
            .find(|attribution| attribution.record_id == "lesson-reject")
            .unwrap();
        assert_eq!(rejected.amount, 0.18);
        assert!(
            rejected
                .reason_codes
                .contains(&"result_rejected".to_owned())
        );
        assert!(
            report
                .attributions
                .iter()
                .all(|attribution| attribution.task_id != "unsafe-sidecar")
        );
        assert!(
            report
                .telemetry
                .iter()
                .any(|line| line == "agent_recall_outcome_attribution_penalize=2")
        );
    }

    #[test]
    fn recall_outcome_attribution_skips_accepted_recall_without_execution_outcome() {
        let task = AgentTask::new(
            "missing-outcome",
            AgentRole::Coder,
            "reuse memory but execution is absent",
            AgentBudget::new(8, 1, 1),
        );
        let recall_context = MemoryRecallContextPlanner::new().plan_from_records(
            &task,
            "role:coder lane:default objective:reuse memory but execution is absent".to_owned(),
            vec![MemoryRecord::new(
                "lesson-missing",
                "needs execution result before attribution",
                "long_term",
            )],
        );
        let recall_plan = AgentWaveMemoryRecallPlan {
            contexts: vec![recall_context],
            telemetry: Vec::new(),
        };
        let execution = AgentWaveExecution {
            results: Vec::new(),
            failures: Vec::new(),
        };

        let report = AgentRecallOutcomeAttributionPlanner::new().plan(&recall_plan, &execution);

        assert!(!report.has_updates());
        assert_eq!(
            report.skipped_missing_outcome_task_ids,
            vec!["missing-outcome".to_owned()]
        );
        assert!(report.telemetry.iter().any(|line| {
            line == "agent_recall_outcome_attribution_skipped_missing_outcome_tasks=1"
        }));
    }

    #[test]
    fn memory_reuse_execution_preflight_allows_clean_read_only_bridge() {
        let task = AgentTask::new(
            "memory-coder",
            AgentRole::Coder,
            "execute after memory reuse preflight",
            AgentBudget::new(8, 1, 1),
        );
        let dispatch = AgentCycleDispatch {
            wave: AgentExecutionWave {
                wave: 0,
                task_ids: vec![task.id.clone()],
                parallel_count: 1,
            },
            dispatch: TaskDispatchPlan {
                assignments: vec![TaskAssignment {
                    task_id: task.id.clone(),
                    role: AgentRole::Coder,
                    lane: "default".to_owned(),
                    budget_reserved: AgentBudget::new(8, 1, 1),
                }],
                ..TaskDispatchPlan::default()
            },
            assigned_tasks: vec![task.clone()],
            blocked_task_ids: Vec::new(),
            remaining_queue: crate::task::AgentTaskQueue::new(),
        };
        let recall_context = MemoryRecallContextPlanner::new().plan_from_records(
            &task,
            "role:coder lane:default objective:execute after memory reuse preflight".to_owned(),
            vec![MemoryRecord::new(
                "lesson-1",
                "safe runtime reuse lesson",
                "long_term",
            )],
        );
        let recall_plan = AgentWaveMemoryRecallPlan {
            contexts: vec![recall_context],
            telemetry: Vec::new(),
        };
        let evidence = MemoryRecallDryRunEvidence {
            source: "norion_memory_reuse_dry_run".to_owned(),
            read_only: true,
            candidate_count: 2,
            long_term_match_count: 2,
            context_decision_count: 2,
            accepted_context_count: 1,
            rejected_context_count: 1,
            used_tokens: 64,
            requested_kv_count: 3,
            kv_promote_count: 1,
            kv_missing_count: 1,
            kv_already_hot_count: 1,
            kv_duplicate_count: 0,
            kv_backend_available: true,
            memory_store_write_allowed: false,
            kv_prefetch_apply_allowed: false,
            reason_codes: vec!["read_only".to_owned(), "context_accepted".to_owned()],
            detail_codes: vec!["kv_prefetch:promote:636f6c64".to_owned()],
        };

        let report = AgentMemoryReuseExecutionPreflightPlanner::new().plan_for_dispatch(
            &dispatch,
            &recall_plan,
            &[evidence],
        );

        assert!(report.read_only);
        assert!(report.memory_reuse_ready);
        assert!(report.can_enter_execution);
        assert!(!report.requires_repair_first);
        assert!(!report.prompt_injection_allowed);
        assert!(!report.engine_port_touched);
        assert!(!report.memory_store_write_allowed);
        assert!(!report.kv_prefetch_apply_allowed);
        assert_eq!(report.task_ids, vec!["memory-coder"]);
        assert_eq!(report.recall_task_ids, vec!["memory-coder"]);
        assert_eq!(report.accepted_recall_count, 1);
        assert_eq!(report.rejected_recall_count, 0);
        assert_eq!(report.dry_run_evidence_count, 1);
        assert_eq!(report.kv_requested_count, 3);
        assert_eq!(report.kv_promote_count, 1);
        assert_eq!(report.kv_missing_count, 1);
        assert!(report.blocked_reasons.is_empty());
        assert!(
            report
                .telemetry
                .iter()
                .any(|line| line == "agent_memory_reuse_execution_preflight_ready=true")
        );
        assert!(report.summary_line().contains("can_enter_execution=true"));
    }

    #[test]
    fn memory_reuse_execution_preflight_blocks_mismatch_and_write_flags() {
        let task = AgentTask::new(
            "memory-coder",
            AgentRole::Coder,
            "execute after memory reuse preflight",
            AgentBudget::new(8, 1, 1),
        );
        let dispatch = AgentCycleDispatch {
            wave: AgentExecutionWave {
                wave: 0,
                task_ids: vec![task.id.clone()],
                parallel_count: 1,
            },
            dispatch: TaskDispatchPlan::default(),
            assigned_tasks: vec![task],
            blocked_task_ids: Vec::new(),
            remaining_queue: crate::task::AgentTaskQueue::new(),
        };
        let unexpected_task = AgentTask::new(
            "other-task",
            AgentRole::Reviewer,
            "wrong sidecar task",
            AgentBudget::new(4, 1, 1),
        );
        let recall_context = MemoryRecallContextPlanner::new().plan_from_records(
            &unexpected_task,
            "role:reviewer lane:default objective:wrong sidecar task".to_owned(),
            vec![MemoryRecord::new(
                "lesson-1",
                "wrong task lesson",
                "long_term",
            )],
        );
        let recall_plan = AgentWaveMemoryRecallPlan {
            contexts: vec![recall_context],
            telemetry: Vec::new(),
        };
        let evidence = MemoryRecallDryRunEvidence {
            source: "unsafe_dry_run".to_owned(),
            read_only: true,
            candidate_count: 1,
            long_term_match_count: 1,
            context_decision_count: 1,
            accepted_context_count: 1,
            rejected_context_count: 0,
            used_tokens: 32,
            requested_kv_count: 1,
            kv_promote_count: 1,
            kv_missing_count: 0,
            kv_already_hot_count: 0,
            kv_duplicate_count: 0,
            kv_backend_available: true,
            memory_store_write_allowed: true,
            kv_prefetch_apply_allowed: true,
            reason_codes: vec!["read_only".to_owned()],
            detail_codes: Vec::new(),
        };

        let report = AgentMemoryReuseExecutionPreflightPlanner::new().plan_for_dispatch(
            &dispatch,
            &recall_plan,
            &[evidence],
        );

        assert!(!report.read_only);
        assert!(!report.memory_reuse_ready);
        assert!(!report.can_enter_execution);
        assert!(report.requires_repair_first);
        assert!(!report.memory_store_write_allowed);
        assert!(!report.kv_prefetch_apply_allowed);
        assert_eq!(report.missing_recall_task_ids, vec!["memory-coder"]);
        assert_eq!(report.unexpected_recall_task_ids, vec!["other-task"]);
        assert_eq!(report.unsafe_dry_run_sources, vec!["unsafe_dry_run"]);
        assert!(
            report
                .blocked_reasons
                .contains(&"memory_reuse_recall_missing_task=memory-coder".to_owned())
        );
        assert!(
            report
                .blocked_reasons
                .contains(&"memory_reuse_recall_unexpected_task=other-task".to_owned())
        );
        assert!(
            report
                .blocked_reasons
                .contains(&"memory_store_write_allowed".to_owned())
        );
        assert!(
            report
                .blocked_reasons
                .contains(&"kv_prefetch_apply_allowed".to_owned())
        );
        assert!(report.summary_line().contains("requires_repair_first=true"));
    }

    #[test]
    fn memory_submission_history_watches_empty() {
        let health =
            MemorySubmissionSummaryHistory::new().health(MemorySubmissionHealthPolicy::default());

        assert_eq!(health.status, MemorySubmissionHealthStatus::Watch);
        assert_eq!(
            health.reasons,
            vec!["memory_submission_history_empty".to_owned()]
        );
        assert_eq!(health.dashboard.total_records, 0);
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert!(
            health
                .dashboard
                .telemetry
                .iter()
                .any(|line| { line == "agent_memory_submission_dashboard_records=0" })
        );
    }

    #[test]
    fn memory_submission_history_marks_clean_submission_stable() {
        let report = MemorySubmissionReport {
            submitted: vec![MemoryNote::new("agent_cycle", "remember clean loop")],
            failures: Vec::new(),
            blocked_reasons: Vec::new(),
            note_quality: None,
        };

        let record = MemorySubmissionSummaryHistoryRecorder::new().record_report_with_health(
            MemorySubmissionSummaryHistory::new(),
            &report,
            MemorySubmissionHealthPolicy::default(),
        );

        assert_eq!(record.history.len(), 1);
        assert_eq!(record.records(), 1);
        assert!(record.appended_summary.clean);
        assert_eq!(record.dashboard.clean_records, 1);
        assert_eq!(record.dashboard.repair_first_records, 0);
        assert_eq!(record.dashboard.submitted_notes, 1);
        assert_eq!(record.dashboard.failed_notes, 0);
        assert_eq!(record.dashboard.blocked_reasons, 0);
        assert_eq!(record.dashboard.clean_rate, 1.0);
        assert_eq!(record.health.status, MemorySubmissionHealthStatus::Stable);
        assert!(record.health.is_stable());
        assert!(record.health.allows_service_advance());
        assert!(!record.health.requires_repair_first());
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "agent_memory_submission_history_record_status=stable" })
        );
    }

    #[test]
    fn memory_submission_history_repairs_failures_and_blockers() {
        let clean_summary = MemorySubmissionSummary {
            submitted_notes: 1,
            failed_notes: 0,
            blocked_reasons: 0,
            attempted_notes: 1,
            quality_reviewed_notes: 0,
            quality_admitted_notes: 0,
            quality_rejected_notes: 0,
            clean: true,
            port_attempted: true,
            telemetry: Vec::new(),
        };
        let dirty_summary = MemorySubmissionSummary {
            submitted_notes: 0,
            failed_notes: 1,
            blocked_reasons: 1,
            attempted_notes: 1,
            quality_reviewed_notes: 0,
            quality_admitted_notes: 0,
            quality_rejected_notes: 0,
            clean: false,
            port_attempted: true,
            telemetry: Vec::new(),
        };
        let history = MemorySubmissionSummaryHistory::from_summaries(vec![clean_summary]);

        let record = MemorySubmissionSummaryHistoryRecorder::new().record_summary_with_health(
            history,
            dirty_summary,
            MemorySubmissionHealthPolicy::default(),
        );

        assert_eq!(record.history.len(), 2);
        assert_eq!(record.dashboard.clean_records, 1);
        assert_eq!(record.dashboard.repair_first_records, 1);
        assert_eq!(record.dashboard.failed_notes, 1);
        assert_eq!(record.dashboard.blocked_reasons, 1);
        assert_eq!(record.dashboard.clean_rate, 0.5);
        assert_eq!(record.health.status, MemorySubmissionHealthStatus::Repair);
        assert!(!record.health.allows_service_advance());
        assert!(record.health.requires_repair_first());
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert_eq!(
            record.health.reasons,
            vec![
                "memory_submission_failed_notes=1>0",
                "memory_submission_blocked_reasons=1>0",
                "memory_submission_clean_rate=0.500<0.67",
            ]
        );
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "agent_memory_submission_history_record_status=repair" })
        );
    }

    fn stable_reflection_gate() -> ReflectionLoopHistoryGateDecision {
        let mut loop_state = ReflectionLoop::new();
        loop_state
            .submit(ReflectionStage::Draft, "draft accepted")
            .unwrap();
        loop_state
            .submit(ReflectionStage::Critique, "no blocker")
            .unwrap();
        loop_state
            .submit(ReflectionStage::Revision, "keep memory evidence")
            .unwrap();
        loop_state
            .submit(ReflectionStage::MemoryNote, "remember clean handoff")
            .unwrap();

        ReflectionLoopSummaryHistoryRecorder::new()
            .record_loop_with_health_gate(
                ReflectionLoopSummaryHistory::new(),
                &loop_state,
                ReflectionLoopHealthPolicy::default(),
            )
            .gate_decision
    }

    fn review_trend_gate(
        messages: Vec<AgentMessage>,
    ) -> AggregationConflictReviewTrendGateDecision {
        let review = AggregationConflictReviewer::new().review_messages(
            messages,
            AggregationSummaryHistory::new(),
            AggregationHealthPolicy::default(),
            ConflictReportSummaryHistory::new(),
            ConflictReportHealthPolicy::default(),
        );
        let record = AggregationConflictReviewSummaryHistoryRecorder::new()
            .record_review_with_health(
                AggregationConflictReviewSummaryHistory::new(),
                &review,
                AggregationConflictReviewHealthPolicy::default(),
            );

        AggregationConflictReviewTrendGate::new().gate(&review, &record)
    }

    fn stable_memory_submission_health() -> MemorySubmissionHealth {
        let report = MemorySubmissionReport {
            submitted: vec![MemoryNote::new("agent_cycle", "remember clean handoff")],
            failures: Vec::new(),
            blocked_reasons: Vec::new(),
            note_quality: None,
        };

        MemorySubmissionSummaryHistoryRecorder::new()
            .record_report_with_health(
                MemorySubmissionSummaryHistory::new(),
                &report,
                MemorySubmissionHealthPolicy::default(),
            )
            .health
    }

    #[test]
    fn memory_promotion_gate_promotes_stable_reflection_review_and_memory_history() {
        let reflection_gate = stable_reflection_gate();
        let review_gate = review_trend_gate(vec![
            AgentMessage::new(
                "m1",
                AgentRole::Researcher,
                AgentMessageKind::Finding,
                "memory",
                "remember the clean handoff",
            ),
            AgentMessage::new(
                "m2",
                AgentRole::Reviewer,
                AgentMessageKind::Finding,
                "budget",
                "budget remained isolated",
            ),
        ]);
        let memory_health = stable_memory_submission_health();
        let notes = vec![MemoryNote::new("agent_cycle", "remember clean handoff")];

        let gate =
            MemoryPromotionGate::new().gate(&notes, &reflection_gate, &review_gate, &memory_health);

        assert_eq!(gate.candidate_notes, 1);
        assert!(gate.can_promote_memory_note);
        assert!(gate.can_submit_memory);
        assert!(gate.is_memory_promotable());
        assert!(!gate.requires_repair_first);
        assert!(gate.repair_tasks.is_empty());
        assert!(gate.reasons.is_empty());
        assert!(
            gate.telemetry
                .iter()
                .any(|line| line == "agent_memory_promotion_gate_promote=true")
        );
    }

    #[test]
    fn memory_note_quality_rejects_empty_and_duplicate_candidates() {
        let notes = vec![
            MemoryNote::new("agent_cycle", "remember budget isolation"),
            MemoryNote::new("  ", "missing topic"),
            MemoryNote::new("agent_cycle", "   "),
            MemoryNote::new(" agent_cycle ", "remember   budget isolation"),
        ];

        let report = MemoryNoteQualityReport::from_notes(&notes);

        assert_eq!(report.admitted_notes, 1);
        assert_eq!(report.rejected_notes, 3);
        assert_eq!(
            report
                .decisions
                .iter()
                .map(|decision| decision.kind)
                .collect::<Vec<_>>(),
            vec![
                MemoryNoteQualityDecisionKind::Admit,
                MemoryNoteQualityDecisionKind::RejectEmptyTopic,
                MemoryNoteQualityDecisionKind::RejectEmptyContent,
                MemoryNoteQualityDecisionKind::RejectDuplicate,
            ]
        );
        assert_eq!(
            report.rejection_reasons(),
            vec![
                "memory_note_quality_empty_topic index=1".to_owned(),
                "memory_note_quality_empty_content index=2".to_owned(),
                "memory_note_quality_duplicate index=3".to_owned(),
            ]
        );
        assert!(
            report
                .telemetry
                .iter()
                .any(|line| line == "agent_memory_note_quality_rejected_notes=3")
        );
    }

    #[test]
    fn memory_promotion_gate_filters_bad_notes_before_submission() {
        let reflection_gate = stable_reflection_gate();
        let review_gate = review_trend_gate(vec![AgentMessage::new(
            "m1",
            AgentRole::Researcher,
            AgentMessageKind::Finding,
            "memory",
            "remember the clean handoff",
        )]);
        let memory_health = stable_memory_submission_health();
        let notes = vec![
            MemoryNote::new("agent_cycle", "remember clean handoff"),
            MemoryNote::new("", "missing topic"),
            MemoryNote::new("agent_cycle", "remember   clean   handoff"),
        ];

        let gate =
            MemoryPromotionGate::new().gate(&notes, &reflection_gate, &review_gate, &memory_health);

        assert_eq!(gate.candidate_notes, 3);
        assert_eq!(gate.admitted_candidate_notes, 1);
        assert_eq!(gate.rejected_candidate_notes, 2);
        assert!(gate.can_promote_memory_note);
        assert!(gate.can_submit_memory);
        assert!(!gate.requires_repair_first);
        assert!(gate.reasons.is_empty());
        assert_eq!(
            gate.note_quality.rejection_reasons(),
            vec![
                "memory_note_quality_empty_topic index=1",
                "memory_note_quality_duplicate index=2",
            ]
        );
        assert!(
            gate.telemetry
                .iter()
                .any(|line| { line == "agent_memory_promotion_gate_admitted_candidate_notes=1" })
        );
        assert!(
            gate.telemetry
                .iter()
                .any(|line| { line == "agent_memory_promotion_gate_rejected_candidate_notes=2" })
        );
    }

    #[test]
    fn memory_promotion_gate_blocks_when_all_candidate_notes_fail_quality() {
        let reflection_gate = stable_reflection_gate();
        let review_gate = review_trend_gate(vec![AgentMessage::new(
            "m1",
            AgentRole::Researcher,
            AgentMessageKind::Finding,
            "memory",
            "reject empty memory notes",
        )]);
        let memory_health = stable_memory_submission_health();
        let notes = vec![
            MemoryNote::new("", "missing topic"),
            MemoryNote::new("agent_cycle", "   "),
        ];

        let gate =
            MemoryPromotionGate::new().gate(&notes, &reflection_gate, &review_gate, &memory_health);

        assert_eq!(gate.candidate_notes, 2);
        assert_eq!(gate.admitted_candidate_notes, 0);
        assert_eq!(gate.rejected_candidate_notes, 2);
        assert!(!gate.can_promote_memory_note);
        assert!(!gate.can_submit_memory);
        assert!(!gate.requires_repair_first);
        assert_eq!(
            gate.reasons,
            vec![
                "memory_promotion_no_admitted_candidate_notes",
                "memory_promotion_note_quality:memory_note_quality_empty_topic index=0",
                "memory_promotion_note_quality:memory_note_quality_empty_content index=1",
            ]
        );
    }

    #[test]
    fn memory_promotion_gate_blocks_unresolved_conflict_before_memory_note() {
        let reflection_gate = stable_reflection_gate();
        let review_gate = review_trend_gate(vec![
            AgentMessage::new(
                "approve",
                AgentRole::Coder,
                AgentMessageKind::Decision,
                "memory",
                "approve memory note and proceed",
            ),
            AgentMessage::new(
                "block",
                AgentRole::Reviewer,
                AgentMessageKind::Risk,
                "memory",
                "reject memory note until validation passes",
            ),
        ]);
        let memory_health = stable_memory_submission_health();
        let notes = vec![MemoryNote::new("agent_cycle", "remember contested note")];

        let gate =
            MemoryPromotionGate::new().gate(&notes, &reflection_gate, &review_gate, &memory_health);

        assert!(!gate.can_promote_memory_note);
        assert!(!gate.can_submit_memory);
        assert!(!gate.is_memory_promotable());
        assert!(gate.requires_repair_first);
        assert!(!gate.repair_tasks.is_empty());
        assert!(gate.reasons.iter().any(|reason| {
            reason
                == "memory_promotion_aggregation_conflict:conflict_report:conflict_report_unresolved_conflicts=1"
        }));
        assert!(
            gate.telemetry
                .iter()
                .any(|line| { line == "agent_memory_promotion_gate_requires_repair_first=true" })
        );
    }

    #[test]
    fn memory_promotion_gate_blocks_empty_candidate_notes_without_repair_first() {
        let reflection_gate = stable_reflection_gate();
        let review_gate = review_trend_gate(vec![AgentMessage::new(
            "m1",
            AgentRole::Researcher,
            AgentMessageKind::Finding,
            "memory",
            "clean handoff had no durable lesson",
        )]);
        let memory_health = stable_memory_submission_health();

        let gate =
            MemoryPromotionGate::new().gate(&[], &reflection_gate, &review_gate, &memory_health);

        assert_eq!(gate.candidate_notes, 0);
        assert!(!gate.can_promote_memory_note);
        assert!(!gate.can_submit_memory);
        assert!(!gate.is_memory_promotable());
        assert!(!gate.requires_repair_first);
        assert!(gate.repair_tasks.is_empty());
        assert_eq!(gate.reasons, vec!["memory_promotion_no_candidate_notes"]);
        assert!(
            gate.telemetry
                .iter()
                .any(|line| line == "agent_memory_promotion_gate_candidate_notes=0")
        );
    }

    #[test]
    fn memory_promotion_gate_keeps_watch_history_observable_but_not_promotable() {
        let reflection_gate = stable_reflection_gate();
        let review_gate = review_trend_gate(vec![AgentMessage::new(
            "m1",
            AgentRole::Researcher,
            AgentMessageKind::Finding,
            "memory",
            "remember clean handoff",
        )]);
        let memory_health =
            MemorySubmissionSummaryHistory::new().health(MemorySubmissionHealthPolicy::default());
        let notes = vec![MemoryNote::new("agent_cycle", "remember clean handoff")];

        let gate =
            MemoryPromotionGate::new().gate(&notes, &reflection_gate, &review_gate, &memory_health);

        assert!(!gate.can_promote_memory_note);
        assert!(!gate.can_submit_memory);
        assert!(!gate.is_memory_promotable());
        assert!(!gate.requires_repair_first);
        assert!(gate.repair_tasks.is_empty());
        assert_eq!(
            gate.reasons,
            vec!["memory_promotion_submission_history:memory_submission_history_empty"]
        );
        assert!(
            gate.telemetry
                .iter()
                .any(|line| line == "agent_memory_promotion_gate_memory_health=watch")
        );
    }

    #[test]
    fn memory_promotion_gate_repairs_dirty_submission_history_before_clean_candidate() {
        let reflection_gate = stable_reflection_gate();
        let review_gate = review_trend_gate(vec![AgentMessage::new(
            "m1",
            AgentRole::Researcher,
            AgentMessageKind::Finding,
            "memory",
            "remember clean handoff after repair",
        )]);
        let dirty_submission = MemorySubmissionReport {
            submitted: Vec::new(),
            failures: Vec::new(),
            blocked_reasons: vec!["unresolved_conflicts=1".to_owned()],
            note_quality: None,
        };
        let memory_health = MemorySubmissionSummaryHistoryRecorder::new()
            .record_report_with_health(
                MemorySubmissionSummaryHistory::new(),
                &dirty_submission,
                MemorySubmissionHealthPolicy::default(),
            )
            .health;
        let notes = vec![MemoryNote::new(
            "agent_cycle",
            "remember clean handoff after repair",
        )];

        let gate =
            MemoryPromotionGate::new().gate(&notes, &reflection_gate, &review_gate, &memory_health);

        assert_eq!(gate.candidate_notes, 1);
        assert_eq!(
            gate.memory_health.status,
            MemorySubmissionHealthStatus::Repair
        );
        assert!(!gate.can_promote_memory_note);
        assert!(!gate.can_submit_memory);
        assert!(!gate.is_memory_promotable());
        assert!(gate.requires_repair_first);
        assert_eq!(
            gate.repair_tasks
                .iter()
                .map(|task| task.id.clone())
                .collect::<Vec<_>>(),
            vec![
                "memory-promotion-repair-0".to_owned(),
                "memory-promotion-repair-1".to_owned()
            ]
        );
        assert_eq!(
            gate.reasons,
            vec![
                "memory_promotion_submission_history:memory_submission_blocked_reasons=1>0",
                "memory_promotion_submission_history:memory_submission_clean_rate=0.000<0.67"
            ]
        );
        assert!(
            gate.telemetry
                .iter()
                .any(|line| { line == "agent_memory_promotion_gate_requires_repair_first=true" })
        );
    }
}
