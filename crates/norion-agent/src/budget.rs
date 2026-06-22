use std::collections::BTreeMap;

use crate::task::{AgentRole, AgentTask};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AgentBudget {
    pub tokens: u32,
    pub steps: u32,
    pub messages: u32,
}

impl AgentBudget {
    pub const fn new(tokens: u32, steps: u32, messages: u32) -> Self {
        Self {
            tokens,
            steps,
            messages,
        }
    }

    pub const fn zero() -> Self {
        Self::new(0, 0, 0)
    }

    pub const fn is_zero(self) -> bool {
        self.tokens == 0 && self.steps == 0 && self.messages == 0
    }

    pub const fn has_depleted_dimension(self) -> bool {
        self.tokens == 0 || self.steps == 0 || self.messages == 0
    }

    pub const fn token_depleted(self) -> bool {
        self.tokens == 0
    }

    pub const fn step_depleted(self) -> bool {
        self.steps == 0
    }

    pub const fn message_depleted(self) -> bool {
        self.messages == 0
    }

    pub const fn is_partially_depleted(self) -> bool {
        self.has_depleted_dimension() && !self.is_zero()
    }

    pub fn fits(self, cost: Self) -> bool {
        self.tokens >= cost.tokens && self.steps >= cost.steps && self.messages >= cost.messages
    }

    pub fn consume(&mut self, cost: Self) -> Result<(), BudgetError> {
        if !self.fits(cost) {
            return Err(BudgetError::Insufficient {
                requested: cost,
                remaining: *self,
            });
        }
        self.tokens -= cost.tokens;
        self.steps -= cost.steps;
        self.messages -= cost.messages;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BudgetPolicy {
    pub reject_zero_budget_tasks: bool,
}

impl Default for BudgetPolicy {
    fn default() -> Self {
        Self::strict()
    }
}

impl BudgetPolicy {
    pub const fn strict() -> Self {
        Self {
            reject_zero_budget_tasks: true,
        }
    }

    pub const fn permissive() -> Self {
        Self {
            reject_zero_budget_tasks: false,
        }
    }

    pub fn validate_task(&self, task: &AgentTask) -> Result<(), BudgetPolicyError> {
        if self.reject_zero_budget_tasks && task.required_budget.is_zero() {
            return Err(BudgetPolicyError::ZeroBudgetTask {
                task_id: task.id.clone(),
                role: task.role.clone(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BudgetPolicyError {
    ZeroBudgetTask { task_id: String, role: AgentRole },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BudgetError {
    MissingRole {
        role: AgentRole,
    },
    Insufficient {
        requested: AgentBudget,
        remaining: AgentBudget,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BudgetLedger {
    remaining: BTreeMap<AgentRole, AgentBudget>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetLedgerSummary {
    pub roles: usize,
    pub zero_budget_roles: usize,
    pub partially_depleted_roles: usize,
    pub token_depleted_roles: usize,
    pub step_depleted_roles: usize,
    pub message_depleted_roles: usize,
    pub total_tokens: u32,
    pub total_steps: u32,
    pub total_messages: u32,
    pub depleted_roles: Vec<AgentRole>,
    pub dimension_depleted_roles: Vec<AgentRole>,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetLedgerHealthStatus {
    Stable,
    Watch,
    Repair,
}

impl BudgetLedgerHealthStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Watch => "watch",
            Self::Repair => "repair",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BudgetLedgerSummaryHistory {
    summaries: Vec<BudgetLedgerSummary>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BudgetLedgerDashboard {
    pub total_records: usize,
    pub roles: usize,
    pub zero_budget_roles: usize,
    pub partially_depleted_roles: usize,
    pub token_depleted_roles: usize,
    pub step_depleted_roles: usize,
    pub message_depleted_roles: usize,
    pub depleted_records: usize,
    pub partial_depletion_records: usize,
    pub total_tokens: u32,
    pub total_steps: u32,
    pub total_messages: u32,
    pub zero_budget_role_rate: f32,
    pub partial_depletion_role_rate: f32,
    pub depleted_record_rate: f32,
    pub partial_depletion_record_rate: f32,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BudgetLedgerHealthPolicy {
    pub maximum_zero_budget_roles: usize,
    pub maximum_partially_depleted_roles: usize,
    pub maximum_depleted_records: usize,
    pub maximum_partial_depletion_records: usize,
    pub minimum_total_tokens: u32,
    pub minimum_total_steps: u32,
    pub minimum_total_messages: u32,
}

impl Default for BudgetLedgerHealthPolicy {
    fn default() -> Self {
        Self {
            maximum_zero_budget_roles: 0,
            maximum_partially_depleted_roles: 0,
            maximum_depleted_records: 0,
            maximum_partial_depletion_records: 0,
            minimum_total_tokens: 1,
            minimum_total_steps: 1,
            minimum_total_messages: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BudgetLedgerHealth {
    pub status: BudgetLedgerHealthStatus,
    pub reasons: Vec<String>,
    pub dashboard: BudgetLedgerDashboard,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BudgetLedgerSummaryHistoryRecord {
    pub history: BudgetLedgerSummaryHistory,
    pub appended_summary: BudgetLedgerSummary,
    pub dashboard: BudgetLedgerDashboard,
    pub health: BudgetLedgerHealth,
    pub telemetry: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct BudgetLedgerSummaryHistoryRecorder;

impl BudgetLedger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_budget(mut self, role: AgentRole, budget: AgentBudget) -> Self {
        self.remaining.insert(role, budget);
        self
    }

    pub fn set_budget(&mut self, role: AgentRole, budget: AgentBudget) {
        self.remaining.insert(role, budget);
    }

    pub fn remaining(&self, role: &AgentRole) -> Option<AgentBudget> {
        self.remaining.get(role).copied()
    }

    pub fn consume(&mut self, role: &AgentRole, cost: AgentBudget) -> Result<(), BudgetError> {
        let Some(remaining) = self.remaining.get_mut(role) else {
            return Err(BudgetError::MissingRole { role: role.clone() });
        };
        remaining.consume(cost)
    }

    pub fn snapshot(&self) -> BTreeMap<AgentRole, AgentBudget> {
        self.remaining.clone()
    }

    pub fn summary(&self) -> BudgetLedgerSummary {
        BudgetLedgerSummary::from_snapshot(&self.remaining)
    }
}

impl BudgetLedgerSummary {
    pub fn from_snapshot(snapshot: &BTreeMap<AgentRole, AgentBudget>) -> Self {
        let roles = snapshot.len();
        let mut zero_budget_roles = 0;
        let mut partially_depleted_roles = 0;
        let mut token_depleted_roles = 0;
        let mut step_depleted_roles = 0;
        let mut message_depleted_roles = 0;
        let mut total_tokens = 0;
        let mut total_steps = 0;
        let mut total_messages = 0;
        let mut depleted_roles = Vec::new();
        let mut dimension_depleted_roles = Vec::new();

        for (role, budget) in snapshot {
            if budget.is_zero() {
                zero_budget_roles += 1;
                depleted_roles.push(role.clone());
            }
            if budget.is_partially_depleted() {
                partially_depleted_roles += 1;
            }
            if budget.token_depleted() {
                token_depleted_roles += 1;
            }
            if budget.step_depleted() {
                step_depleted_roles += 1;
            }
            if budget.message_depleted() {
                message_depleted_roles += 1;
            }
            if budget.has_depleted_dimension() {
                dimension_depleted_roles.push(role.clone());
            }
            total_tokens += budget.tokens;
            total_steps += budget.steps;
            total_messages += budget.messages;
        }

        let telemetry = budget_ledger_summary_telemetry(
            roles,
            zero_budget_roles,
            partially_depleted_roles,
            token_depleted_roles,
            step_depleted_roles,
            message_depleted_roles,
            total_tokens,
            total_steps,
            total_messages,
        );

        Self {
            roles,
            zero_budget_roles,
            partially_depleted_roles,
            token_depleted_roles,
            step_depleted_roles,
            message_depleted_roles,
            total_tokens,
            total_steps,
            total_messages,
            depleted_roles,
            dimension_depleted_roles,
            telemetry,
        }
    }
}

impl BudgetLedgerSummaryHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_summaries(summaries: Vec<BudgetLedgerSummary>) -> Self {
        Self { summaries }
    }

    pub fn push(&mut self, summary: BudgetLedgerSummary) {
        self.summaries.push(summary);
    }

    pub fn latest(&self) -> Option<&BudgetLedgerSummary> {
        self.summaries.last()
    }

    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn summaries(&self) -> &[BudgetLedgerSummary] {
        &self.summaries
    }

    pub fn dashboard(&self) -> BudgetLedgerDashboard {
        BudgetLedgerDashboard::from_summaries(&self.summaries)
    }

    pub fn health(&self, policy: BudgetLedgerHealthPolicy) -> BudgetLedgerHealth {
        self.dashboard().health(policy)
    }
}

impl BudgetLedgerDashboard {
    pub fn from_summaries(summaries: &[BudgetLedgerSummary]) -> Self {
        let total_records = summaries.len();
        let roles = summaries.iter().map(|summary| summary.roles).sum::<usize>();
        let zero_budget_roles = summaries
            .iter()
            .map(|summary| summary.zero_budget_roles)
            .sum::<usize>();
        let partially_depleted_roles = summaries
            .iter()
            .map(|summary| summary.partially_depleted_roles)
            .sum::<usize>();
        let token_depleted_roles = summaries
            .iter()
            .map(|summary| summary.token_depleted_roles)
            .sum::<usize>();
        let step_depleted_roles = summaries
            .iter()
            .map(|summary| summary.step_depleted_roles)
            .sum::<usize>();
        let message_depleted_roles = summaries
            .iter()
            .map(|summary| summary.message_depleted_roles)
            .sum::<usize>();
        let depleted_records = summaries
            .iter()
            .filter(|summary| summary.zero_budget_roles > 0)
            .count();
        let partial_depletion_records = summaries
            .iter()
            .filter(|summary| summary.partially_depleted_roles > 0)
            .count();
        let total_tokens = summaries
            .iter()
            .map(|summary| summary.total_tokens)
            .sum::<u32>();
        let total_steps = summaries
            .iter()
            .map(|summary| summary.total_steps)
            .sum::<u32>();
        let total_messages = summaries
            .iter()
            .map(|summary| summary.total_messages)
            .sum::<u32>();
        let zero_budget_role_rate = rate(zero_budget_roles, roles);
        let partial_depletion_role_rate = rate(partially_depleted_roles, roles);
        let depleted_record_rate = rate(depleted_records, total_records);
        let partial_depletion_record_rate = rate(partial_depletion_records, total_records);
        let telemetry = budget_ledger_dashboard_telemetry(
            total_records,
            roles,
            zero_budget_roles,
            partially_depleted_roles,
            token_depleted_roles,
            step_depleted_roles,
            message_depleted_roles,
            depleted_records,
            partial_depletion_records,
            total_tokens,
            total_steps,
            total_messages,
            zero_budget_role_rate,
            partial_depletion_role_rate,
            depleted_record_rate,
            partial_depletion_record_rate,
        );

        Self {
            total_records,
            roles,
            zero_budget_roles,
            partially_depleted_roles,
            token_depleted_roles,
            step_depleted_roles,
            message_depleted_roles,
            depleted_records,
            partial_depletion_records,
            total_tokens,
            total_steps,
            total_messages,
            zero_budget_role_rate,
            partial_depletion_role_rate,
            depleted_record_rate,
            partial_depletion_record_rate,
            telemetry,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_records == 0
    }

    pub fn health(&self, policy: BudgetLedgerHealthPolicy) -> BudgetLedgerHealth {
        BudgetLedgerHealth::from_dashboard(self.clone(), policy)
    }
}

impl BudgetLedgerHealth {
    pub fn from_dashboard(
        dashboard: BudgetLedgerDashboard,
        policy: BudgetLedgerHealthPolicy,
    ) -> Self {
        let mut repair_reasons = Vec::new();
        let mut watch_reasons = Vec::new();

        if dashboard.is_empty() {
            watch_reasons.push("budget_ledger_history_empty".to_owned());
        }

        if dashboard.zero_budget_roles > policy.maximum_zero_budget_roles {
            repair_reasons.push(format!(
                "budget_ledger_zero_budget_roles={}>{}",
                dashboard.zero_budget_roles, policy.maximum_zero_budget_roles
            ));
        }

        if dashboard.partially_depleted_roles > policy.maximum_partially_depleted_roles {
            repair_reasons.push(format!(
                "budget_ledger_partially_depleted_roles={}>{}",
                dashboard.partially_depleted_roles, policy.maximum_partially_depleted_roles
            ));
        }

        if dashboard.depleted_records > policy.maximum_depleted_records {
            repair_reasons.push(format!(
                "budget_ledger_depleted_records={}>{}",
                dashboard.depleted_records, policy.maximum_depleted_records
            ));
        }

        if dashboard.partial_depletion_records > policy.maximum_partial_depletion_records {
            repair_reasons.push(format!(
                "budget_ledger_partial_depletion_records={}>{}",
                dashboard.partial_depletion_records, policy.maximum_partial_depletion_records
            ));
        }

        if !dashboard.is_empty() && dashboard.total_tokens < policy.minimum_total_tokens {
            watch_reasons.push(format!(
                "budget_ledger_total_tokens={}<{}",
                dashboard.total_tokens, policy.minimum_total_tokens
            ));
        }

        if !dashboard.is_empty() && dashboard.total_steps < policy.minimum_total_steps {
            watch_reasons.push(format!(
                "budget_ledger_total_steps={}<{}",
                dashboard.total_steps, policy.minimum_total_steps
            ));
        }

        if !dashboard.is_empty() && dashboard.total_messages < policy.minimum_total_messages {
            watch_reasons.push(format!(
                "budget_ledger_total_messages={}<{}",
                dashboard.total_messages, policy.minimum_total_messages
            ));
        }

        let (status, reasons) = if !repair_reasons.is_empty() {
            repair_reasons.extend(watch_reasons);
            (BudgetLedgerHealthStatus::Repair, repair_reasons)
        } else if !watch_reasons.is_empty() {
            (BudgetLedgerHealthStatus::Watch, watch_reasons)
        } else {
            (BudgetLedgerHealthStatus::Stable, Vec::new())
        };

        Self {
            status,
            reasons,
            dashboard,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.status == BudgetLedgerHealthStatus::Stable
    }

    pub fn allows_service_advance(&self) -> bool {
        self.status != BudgetLedgerHealthStatus::Repair
    }

    pub fn requires_repair_first(&self) -> bool {
        self.status == BudgetLedgerHealthStatus::Repair
    }
}

impl BudgetLedgerSummaryHistoryRecord {
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

impl BudgetLedgerSummaryHistoryRecorder {
    pub fn new() -> Self {
        Self
    }

    pub fn record_summary_with_health(
        &self,
        mut history: BudgetLedgerSummaryHistory,
        summary: BudgetLedgerSummary,
        policy: BudgetLedgerHealthPolicy,
    ) -> BudgetLedgerSummaryHistoryRecord {
        history.push(summary.clone());
        let dashboard = history.dashboard();
        let health = dashboard.health(policy);
        let telemetry = budget_ledger_history_record_telemetry(&dashboard, &health);

        BudgetLedgerSummaryHistoryRecord {
            history,
            appended_summary: summary,
            dashboard,
            health,
            telemetry,
        }
    }

    pub fn record_ledger_with_health(
        &self,
        history: BudgetLedgerSummaryHistory,
        ledger: &BudgetLedger,
        policy: BudgetLedgerHealthPolicy,
    ) -> BudgetLedgerSummaryHistoryRecord {
        self.record_summary_with_health(history, ledger.summary(), policy)
    }

    pub fn record_ledger_with_health_gate(
        &self,
        history: BudgetLedgerSummaryHistory,
        ledger: &BudgetLedger,
        policy: BudgetLedgerHealthPolicy,
    ) -> BudgetLedgerHistoryGateRecord {
        let health_record = self.record_ledger_with_health(history, ledger, policy);
        let gate_decision = BudgetLedgerHistoryGate::new().gate(ledger, &health_record);
        let telemetry = budget_ledger_history_gate_record_telemetry(&health_record, &gate_decision);

        BudgetLedgerHistoryGateRecord {
            health_record,
            gate_decision,
            telemetry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BudgetLedgerHistoryGateDecision {
    pub ledger_summary: BudgetLedgerSummary,
    pub budget_health: BudgetLedgerHealth,
    pub can_dispatch_tasks: bool,
    pub can_promote_side_effects: bool,
    pub requires_repair_first: bool,
    pub repair_tasks: Vec<AgentTask>,
    pub reasons: Vec<String>,
    pub telemetry: Vec<String>,
}

impl BudgetLedgerHistoryGateDecision {
    pub fn is_dispatchable(&self) -> bool {
        self.can_dispatch_tasks && !self.requires_repair_first
    }

    pub fn is_side_effect_safe(&self) -> bool {
        self.can_promote_side_effects && !self.requires_repair_first
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BudgetLedgerHistoryGateRecord {
    pub health_record: BudgetLedgerSummaryHistoryRecord,
    pub gate_decision: BudgetLedgerHistoryGateDecision,
    pub telemetry: Vec<String>,
}

impl BudgetLedgerHistoryGateRecord {
    pub fn records(&self) -> usize {
        self.health_record.records()
    }

    pub fn allows_service_advance(&self) -> bool {
        self.health_record.allows_service_advance()
    }

    pub fn requires_repair_first(&self) -> bool {
        self.gate_decision.requires_repair_first
    }

    pub fn can_dispatch_tasks(&self) -> bool {
        self.gate_decision.can_dispatch_tasks
    }

    pub fn can_promote_side_effects(&self) -> bool {
        self.gate_decision.can_promote_side_effects
    }
}

#[derive(Debug, Clone, Default)]
pub struct BudgetLedgerHistoryGate;

impl BudgetLedgerHistoryGate {
    pub fn new() -> Self {
        Self
    }

    pub fn gate(
        &self,
        ledger: &BudgetLedger,
        history_record: &BudgetLedgerSummaryHistoryRecord,
    ) -> BudgetLedgerHistoryGateDecision {
        let ledger_summary = ledger.summary();
        let budget_health = history_record.health.clone();
        let mut reasons = budget_ledger_gate_reasons(&ledger_summary);
        extend_ordered_unique(
            &mut reasons,
            budget_health
                .reasons
                .iter()
                .map(|reason| format!("budget_ledger_history:{reason}"))
                .collect::<Vec<_>>(),
        );
        let current_requires_repair =
            ledger_summary.zero_budget_roles > 0 || ledger_summary.partially_depleted_roles > 0;
        let requires_repair_first =
            current_requires_repair || budget_health.requires_repair_first();
        let has_dispatch_budget = ledger_summary.roles > 0
            && ledger_summary.total_tokens > 0
            && ledger_summary.total_steps > 0
            && ledger_summary.total_messages > 0;
        let can_dispatch_tasks =
            has_dispatch_budget && budget_health.allows_service_advance() && !requires_repair_first;
        let can_promote_side_effects = can_dispatch_tasks
            && ledger_summary.zero_budget_roles == 0
            && ledger_summary.partially_depleted_roles == 0
            && reasons.is_empty();
        let repair_tasks = budget_ledger_history_gate_repair_tasks(requires_repair_first, &reasons);
        let telemetry = budget_ledger_history_gate_telemetry(
            can_dispatch_tasks,
            can_promote_side_effects,
            requires_repair_first,
            repair_tasks.len(),
            reasons.len(),
            &ledger_summary,
            budget_health.status,
        );

        BudgetLedgerHistoryGateDecision {
            ledger_summary,
            budget_health,
            can_dispatch_tasks,
            can_promote_side_effects,
            requires_repair_first,
            repair_tasks,
            reasons,
            telemetry,
        }
    }
}

fn budget_ledger_summary_telemetry(
    roles: usize,
    zero_budget_roles: usize,
    partially_depleted_roles: usize,
    token_depleted_roles: usize,
    step_depleted_roles: usize,
    message_depleted_roles: usize,
    total_tokens: u32,
    total_steps: u32,
    total_messages: u32,
) -> Vec<String> {
    vec![
        "agent_budget_ledger_summary=true".to_owned(),
        format!("agent_budget_ledger_summary_roles={roles}"),
        format!("agent_budget_ledger_summary_zero_budget_roles={zero_budget_roles}"),
        format!("agent_budget_ledger_summary_partially_depleted_roles={partially_depleted_roles}"),
        format!("agent_budget_ledger_summary_token_depleted_roles={token_depleted_roles}"),
        format!("agent_budget_ledger_summary_step_depleted_roles={step_depleted_roles}"),
        format!("agent_budget_ledger_summary_message_depleted_roles={message_depleted_roles}"),
        format!("agent_budget_ledger_summary_total_tokens={total_tokens}"),
        format!("agent_budget_ledger_summary_total_steps={total_steps}"),
        format!("agent_budget_ledger_summary_total_messages={total_messages}"),
    ]
}

#[allow(clippy::too_many_arguments)]
fn budget_ledger_dashboard_telemetry(
    total_records: usize,
    roles: usize,
    zero_budget_roles: usize,
    partially_depleted_roles: usize,
    token_depleted_roles: usize,
    step_depleted_roles: usize,
    message_depleted_roles: usize,
    depleted_records: usize,
    partial_depletion_records: usize,
    total_tokens: u32,
    total_steps: u32,
    total_messages: u32,
    zero_budget_role_rate: f32,
    partial_depletion_role_rate: f32,
    depleted_record_rate: f32,
    partial_depletion_record_rate: f32,
) -> Vec<String> {
    vec![
        "agent_budget_ledger_dashboard=true".to_owned(),
        format!("agent_budget_ledger_dashboard_records={total_records}"),
        format!("agent_budget_ledger_dashboard_roles={roles}"),
        format!("agent_budget_ledger_dashboard_zero_budget_roles={zero_budget_roles}"),
        format!(
            "agent_budget_ledger_dashboard_partially_depleted_roles={partially_depleted_roles}"
        ),
        format!("agent_budget_ledger_dashboard_token_depleted_roles={token_depleted_roles}"),
        format!("agent_budget_ledger_dashboard_step_depleted_roles={step_depleted_roles}"),
        format!("agent_budget_ledger_dashboard_message_depleted_roles={message_depleted_roles}"),
        format!("agent_budget_ledger_dashboard_depleted_records={depleted_records}"),
        format!(
            "agent_budget_ledger_dashboard_partial_depletion_records={partial_depletion_records}"
        ),
        format!("agent_budget_ledger_dashboard_total_tokens={total_tokens}"),
        format!("agent_budget_ledger_dashboard_total_steps={total_steps}"),
        format!("agent_budget_ledger_dashboard_total_messages={total_messages}"),
        format!("agent_budget_ledger_dashboard_zero_budget_role_rate={zero_budget_role_rate:.3}"),
        format!(
            "agent_budget_ledger_dashboard_partial_depletion_role_rate={partial_depletion_role_rate:.3}"
        ),
        format!("agent_budget_ledger_dashboard_depleted_record_rate={depleted_record_rate:.3}"),
        format!(
            "agent_budget_ledger_dashboard_partial_depletion_record_rate={partial_depletion_record_rate:.3}"
        ),
    ]
}

fn budget_ledger_history_record_telemetry(
    dashboard: &BudgetLedgerDashboard,
    health: &BudgetLedgerHealth,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_budget_ledger_history_record=true".to_owned(),
        format!(
            "agent_budget_ledger_history_record_status={}",
            health.status.as_str()
        ),
        format!(
            "agent_budget_ledger_history_record_records={}",
            dashboard.total_records
        ),
        format!(
            "agent_budget_ledger_history_record_zero_budget_roles={}",
            dashboard.zero_budget_roles
        ),
        format!(
            "agent_budget_ledger_history_record_partially_depleted_roles={}",
            dashboard.partially_depleted_roles
        ),
        format!(
            "agent_budget_ledger_history_record_token_depleted_roles={}",
            dashboard.token_depleted_roles
        ),
        format!(
            "agent_budget_ledger_history_record_step_depleted_roles={}",
            dashboard.step_depleted_roles
        ),
        format!(
            "agent_budget_ledger_history_record_message_depleted_roles={}",
            dashboard.message_depleted_roles
        ),
        format!(
            "agent_budget_ledger_history_record_total_tokens={}",
            dashboard.total_tokens
        ),
    ];
    telemetry.extend(
        health
            .reasons
            .iter()
            .map(|reason| format!("agent_budget_ledger_history_record_reason={reason}")),
    );
    telemetry
}

fn budget_ledger_gate_reasons(summary: &BudgetLedgerSummary) -> Vec<String> {
    let mut reasons = Vec::new();
    if summary.zero_budget_roles > 0 {
        reasons.push(format!(
            "budget_ledger_zero_budget_roles={}",
            summary.zero_budget_roles
        ));
    }
    if summary.partially_depleted_roles > 0 {
        reasons.push(format!(
            "budget_ledger_partially_depleted_roles={}",
            summary.partially_depleted_roles
        ));
    }
    reasons
}

fn budget_ledger_history_gate_repair_tasks(
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
                format!("budget-ledger-repair-{index}"),
                AgentRole::Planner,
                format!("repair budget ledger: {reason}"),
                AgentBudget::new(12, 1, 1),
            )
            .with_lane("budget-ledger-repair")
            .with_priority(1)
        })
        .collect()
}

fn budget_ledger_history_gate_telemetry(
    can_dispatch_tasks: bool,
    can_promote_side_effects: bool,
    requires_repair_first: bool,
    repair_tasks: usize,
    reasons: usize,
    summary: &BudgetLedgerSummary,
    health_status: BudgetLedgerHealthStatus,
) -> Vec<String> {
    vec![
        "agent_budget_ledger_history_gate=true".to_owned(),
        format!(
            "agent_budget_ledger_history_gate_health={}",
            health_status.as_str()
        ),
        format!("agent_budget_ledger_history_gate_dispatch={can_dispatch_tasks}"),
        format!("agent_budget_ledger_history_gate_promote_side_effects={can_promote_side_effects}"),
        format!("agent_budget_ledger_history_gate_requires_repair_first={requires_repair_first}"),
        format!("agent_budget_ledger_history_gate_repair_tasks={repair_tasks}"),
        format!("agent_budget_ledger_history_gate_reasons={reasons}"),
        format!("agent_budget_ledger_history_gate_roles={}", summary.roles),
        format!(
            "agent_budget_ledger_history_gate_zero_budget_roles={}",
            summary.zero_budget_roles
        ),
        format!(
            "agent_budget_ledger_history_gate_partially_depleted_roles={}",
            summary.partially_depleted_roles
        ),
        format!(
            "agent_budget_ledger_history_gate_token_depleted_roles={}",
            summary.token_depleted_roles
        ),
        format!(
            "agent_budget_ledger_history_gate_step_depleted_roles={}",
            summary.step_depleted_roles
        ),
        format!(
            "agent_budget_ledger_history_gate_message_depleted_roles={}",
            summary.message_depleted_roles
        ),
        format!(
            "agent_budget_ledger_history_gate_total_tokens={}",
            summary.total_tokens
        ),
    ]
}

fn budget_ledger_history_gate_record_telemetry(
    health_record: &BudgetLedgerSummaryHistoryRecord,
    gate_decision: &BudgetLedgerHistoryGateDecision,
) -> Vec<String> {
    let mut telemetry = vec![
        "agent_budget_ledger_history_gate_record=true".to_owned(),
        format!(
            "agent_budget_ledger_history_gate_record_health={}",
            health_record.health.status.as_str()
        ),
        format!(
            "agent_budget_ledger_history_gate_record_records={}",
            health_record.records()
        ),
        format!(
            "agent_budget_ledger_history_gate_record_dispatch={}",
            gate_decision.can_dispatch_tasks
        ),
        format!(
            "agent_budget_ledger_history_gate_record_promote_side_effects={}",
            gate_decision.can_promote_side_effects
        ),
        format!(
            "agent_budget_ledger_history_gate_record_requires_repair_first={}",
            gate_decision.requires_repair_first
        ),
        format!(
            "agent_budget_ledger_history_gate_record_repair_tasks={}",
            gate_decision.repair_tasks.len()
        ),
    ];
    telemetry.extend(gate_decision.telemetry.iter().cloned());
    telemetry
}

fn extend_ordered_unique(target: &mut Vec<String>, items: Vec<String>) {
    for item in items {
        if !target.contains(&item) {
            target.push(item);
        }
    }
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
    use crate::task::{AgentRole, AgentTask, DispatchPlanner};

    #[test]
    fn budget_isolated_by_agent_role() {
        let mut planner = DispatchPlanner::new(
            BudgetLedger::new()
                .with_budget(AgentRole::Coder, AgentBudget::new(100, 4, 4))
                .with_budget(AgentRole::Reviewer, AgentBudget::new(20, 2, 2)),
        );
        let tasks = vec![
            AgentTask::new(
                "code",
                AgentRole::Coder,
                "draft isolated crate",
                AgentBudget::new(70, 2, 1),
            ),
            AgentTask::new(
                "review",
                AgentRole::Reviewer,
                "review every line",
                AgentBudget::new(30, 1, 1),
            ),
        ];

        let plan = planner.plan(tasks);

        assert_eq!(plan.assignments.len(), 1);
        assert_eq!(plan.assignments[0].task_id, "code");
        assert_eq!(plan.rejections.len(), 1);
        assert_eq!(plan.rejections[0].task_id, "review");
        assert_eq!(
            plan.remaining.get(&AgentRole::Coder).copied(),
            Some(AgentBudget::new(30, 2, 3))
        );
        assert_eq!(
            plan.remaining.get(&AgentRole::Reviewer).copied(),
            Some(AgentBudget::new(20, 2, 2))
        );
    }

    #[test]
    fn budget_exhaustion_rejects_without_consuming_remaining_budget() {
        let mut planner = DispatchPlanner::new(
            BudgetLedger::new().with_budget(AgentRole::Tester, AgentBudget::new(10, 1, 1)),
        );
        let task = AgentTask::new(
            "heavy-test",
            AgentRole::Tester,
            "run the whole suite",
            AgentBudget::new(11, 1, 1),
        );

        let plan = planner.plan(vec![task]);

        assert!(plan.assignments.is_empty());
        assert_eq!(plan.rejections.len(), 1);
        assert_eq!(plan.rejections[0].task_id, "heavy-test");
        assert!(plan.rejections[0].reason.contains("insufficient budget"));
        assert_eq!(
            plan.remaining.get(&AgentRole::Tester).copied(),
            Some(AgentBudget::new(10, 1, 1))
        );
    }

    #[test]
    fn strict_budget_policy_rejects_zero_budget_tasks() {
        let task = AgentTask::new(
            "noop",
            AgentRole::Planner,
            "unbudgeted coordination",
            AgentBudget::zero(),
        );

        let rejection = BudgetPolicy::strict().validate_task(&task).unwrap_err();

        assert_eq!(
            rejection,
            BudgetPolicyError::ZeroBudgetTask {
                task_id: "noop".to_owned(),
                role: AgentRole::Planner
            }
        );
    }

    #[test]
    fn budget_ledger_history_watches_empty() {
        let health = BudgetLedgerSummaryHistory::new().health(BudgetLedgerHealthPolicy::default());

        assert_eq!(health.status, BudgetLedgerHealthStatus::Watch);
        assert!(!health.is_stable());
        assert!(health.allows_service_advance());
        assert!(!health.requires_repair_first());
        assert_eq!(
            health.reasons,
            vec!["budget_ledger_history_empty".to_owned()]
        );
        assert_eq!(health.dashboard.total_records, 0);
        assert!(
            health
                .dashboard
                .telemetry
                .iter()
                .any(|line| { line == "agent_budget_ledger_dashboard_records=0" })
        );
    }

    #[test]
    fn budget_ledger_history_marks_available_budget_stable() {
        let ledger = BudgetLedger::new()
            .with_budget(AgentRole::Coder, AgentBudget::new(10, 2, 2))
            .with_budget(AgentRole::Reviewer, AgentBudget::new(5, 1, 1));

        let record = BudgetLedgerSummaryHistoryRecorder::new().record_ledger_with_health(
            BudgetLedgerSummaryHistory::new(),
            &ledger,
            BudgetLedgerHealthPolicy::default(),
        );

        assert_eq!(record.records(), 1);
        assert_eq!(record.appended_summary.roles, 2);
        assert_eq!(record.appended_summary.zero_budget_roles, 0);
        assert_eq!(record.appended_summary.partially_depleted_roles, 0);
        assert_eq!(record.appended_summary.token_depleted_roles, 0);
        assert_eq!(record.appended_summary.step_depleted_roles, 0);
        assert_eq!(record.appended_summary.message_depleted_roles, 0);
        assert_eq!(record.dashboard.total_tokens, 15);
        assert_eq!(record.dashboard.total_steps, 3);
        assert_eq!(record.dashboard.total_messages, 3);
        assert_eq!(record.dashboard.zero_budget_roles, 0);
        assert_eq!(record.dashboard.partially_depleted_roles, 0);
        assert_eq!(record.dashboard.token_depleted_roles, 0);
        assert_eq!(record.dashboard.step_depleted_roles, 0);
        assert_eq!(record.dashboard.message_depleted_roles, 0);
        assert_eq!(record.dashboard.depleted_records, 0);
        assert_eq!(record.dashboard.partial_depletion_records, 0);
        assert_eq!(record.health.status, BudgetLedgerHealthStatus::Stable);
        assert!(record.health.is_stable());
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "agent_budget_ledger_history_record_status=stable" })
        );
    }

    #[test]
    fn budget_ledger_history_repairs_depleted_roles() {
        let clean = BudgetLedgerSummary {
            roles: 1,
            zero_budget_roles: 0,
            partially_depleted_roles: 0,
            token_depleted_roles: 0,
            step_depleted_roles: 0,
            message_depleted_roles: 0,
            total_tokens: 10,
            total_steps: 1,
            total_messages: 1,
            depleted_roles: Vec::new(),
            dimension_depleted_roles: Vec::new(),
            telemetry: Vec::new(),
        };
        let depleted = BudgetLedgerSummary {
            roles: 1,
            zero_budget_roles: 1,
            partially_depleted_roles: 0,
            token_depleted_roles: 1,
            step_depleted_roles: 1,
            message_depleted_roles: 1,
            total_tokens: 0,
            total_steps: 0,
            total_messages: 0,
            depleted_roles: vec![AgentRole::Tester],
            dimension_depleted_roles: vec![AgentRole::Tester],
            telemetry: Vec::new(),
        };
        let history = BudgetLedgerSummaryHistory::from_summaries(vec![clean]);

        let record = BudgetLedgerSummaryHistoryRecorder::new().record_summary_with_health(
            history,
            depleted,
            BudgetLedgerHealthPolicy::default(),
        );

        assert_eq!(record.records(), 2);
        assert_eq!(record.dashboard.roles, 2);
        assert_eq!(record.dashboard.zero_budget_roles, 1);
        assert_eq!(record.dashboard.partially_depleted_roles, 0);
        assert_eq!(record.dashboard.token_depleted_roles, 1);
        assert_eq!(record.dashboard.step_depleted_roles, 1);
        assert_eq!(record.dashboard.message_depleted_roles, 1);
        assert_eq!(record.dashboard.depleted_records, 1);
        assert_eq!(record.dashboard.partial_depletion_records, 0);
        assert_eq!(record.dashboard.zero_budget_role_rate, 0.5);
        assert_eq!(record.dashboard.partial_depletion_role_rate, 0.0);
        assert_eq!(record.health.status, BudgetLedgerHealthStatus::Repair);
        assert!(!record.health.is_stable());
        assert!(!record.allows_service_advance());
        assert!(record.requires_repair_first());
        assert_eq!(
            record.health.reasons,
            vec![
                "budget_ledger_zero_budget_roles=1>0",
                "budget_ledger_depleted_records=1>0",
            ]
        );
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "agent_budget_ledger_history_record_status=repair" })
        );
    }

    #[test]
    fn partial_budget_depletion_repairs_partially_depleted_roles() {
        let ledger = BudgetLedger::new()
            .with_budget(AgentRole::Coder, AgentBudget::new(10, 0, 1))
            .with_budget(AgentRole::Reviewer, AgentBudget::new(5, 1, 1));

        let record = BudgetLedgerSummaryHistoryRecorder::new().record_ledger_with_health(
            BudgetLedgerSummaryHistory::new(),
            &ledger,
            BudgetLedgerHealthPolicy::default(),
        );

        assert_eq!(record.appended_summary.zero_budget_roles, 0);
        assert_eq!(record.appended_summary.partially_depleted_roles, 1);
        assert_eq!(record.appended_summary.token_depleted_roles, 0);
        assert_eq!(record.appended_summary.step_depleted_roles, 1);
        assert_eq!(record.appended_summary.message_depleted_roles, 0);
        assert_eq!(
            record.appended_summary.depleted_roles,
            Vec::<AgentRole>::new()
        );
        assert_eq!(
            record.appended_summary.dimension_depleted_roles,
            vec![AgentRole::Coder]
        );
        assert_eq!(record.dashboard.zero_budget_roles, 0);
        assert_eq!(record.dashboard.partially_depleted_roles, 1);
        assert_eq!(record.dashboard.token_depleted_roles, 0);
        assert_eq!(record.dashboard.step_depleted_roles, 1);
        assert_eq!(record.dashboard.message_depleted_roles, 0);
        assert_eq!(record.dashboard.depleted_records, 0);
        assert_eq!(record.dashboard.partial_depletion_records, 1);
        assert_eq!(record.dashboard.partial_depletion_role_rate, 0.5);
        assert_eq!(record.health.status, BudgetLedgerHealthStatus::Repair);
        assert_eq!(
            record.health.reasons,
            vec![
                "budget_ledger_partially_depleted_roles=1>0",
                "budget_ledger_partial_depletion_records=1>0",
            ]
        );
        assert!(record.telemetry.iter().any(|line| {
            line == "agent_budget_ledger_history_record_partially_depleted_roles=1"
        }));
    }

    #[test]
    fn partial_budget_depletion_classifies_each_budget_axis() {
        let ledger = BudgetLedger::new()
            .with_budget(AgentRole::Coder, AgentBudget::new(0, 1, 1))
            .with_budget(AgentRole::Reviewer, AgentBudget::new(1, 0, 1))
            .with_budget(AgentRole::Tester, AgentBudget::new(1, 1, 0))
            .with_budget(AgentRole::Reflector, AgentBudget::zero());

        let summary = ledger.summary();

        assert_eq!(summary.roles, 4);
        assert_eq!(summary.zero_budget_roles, 1);
        assert_eq!(summary.partially_depleted_roles, 3);
        assert_eq!(summary.token_depleted_roles, 2);
        assert_eq!(summary.step_depleted_roles, 2);
        assert_eq!(summary.message_depleted_roles, 2);
        assert_eq!(summary.depleted_roles, vec![AgentRole::Reflector]);
        assert_eq!(summary.dimension_depleted_roles.len(), 4);
        assert!(
            summary
                .telemetry
                .iter()
                .any(|line| { line == "agent_budget_ledger_summary_token_depleted_roles=2" })
        );
        assert!(
            summary
                .telemetry
                .iter()
                .any(|line| { line == "agent_budget_ledger_summary_step_depleted_roles=2" })
        );
        assert!(
            summary
                .telemetry
                .iter()
                .any(|line| { line == "agent_budget_ledger_summary_message_depleted_roles=2" })
        );
    }

    #[test]
    fn budget_ledger_history_gate_allows_available_budget() {
        let ledger = BudgetLedger::new()
            .with_budget(AgentRole::Coder, AgentBudget::new(10, 2, 2))
            .with_budget(AgentRole::Reviewer, AgentBudget::new(5, 1, 1));
        let history_record = BudgetLedgerSummaryHistoryRecorder::new().record_ledger_with_health(
            BudgetLedgerSummaryHistory::new(),
            &ledger,
            BudgetLedgerHealthPolicy::default(),
        );

        let gate = BudgetLedgerHistoryGate::new().gate(&ledger, &history_record);

        assert!(gate.can_dispatch_tasks);
        assert!(gate.can_promote_side_effects);
        assert!(gate.is_dispatchable());
        assert!(gate.is_side_effect_safe());
        assert!(!gate.requires_repair_first);
        assert!(gate.repair_tasks.is_empty());
        assert!(gate.reasons.is_empty());
        assert_eq!(gate.budget_health.status, BudgetLedgerHealthStatus::Stable);
        assert!(
            gate.telemetry
                .iter()
                .any(|line| { line == "agent_budget_ledger_history_gate_dispatch=true" })
        );
    }

    #[test]
    fn budget_ledger_history_gate_repairs_current_depleted_roles() {
        let clean = BudgetLedgerSummary {
            roles: 1,
            zero_budget_roles: 0,
            partially_depleted_roles: 0,
            token_depleted_roles: 0,
            step_depleted_roles: 0,
            message_depleted_roles: 0,
            total_tokens: 10,
            total_steps: 1,
            total_messages: 1,
            depleted_roles: Vec::new(),
            dimension_depleted_roles: Vec::new(),
            telemetry: Vec::new(),
        };
        let ledger = BudgetLedger::new().with_budget(AgentRole::Tester, AgentBudget::new(0, 0, 0));
        let history_record = BudgetLedgerSummaryHistoryRecorder::new().record_ledger_with_health(
            BudgetLedgerSummaryHistory::from_summaries(vec![clean]),
            &ledger,
            BudgetLedgerHealthPolicy::default(),
        );

        let gate = BudgetLedgerHistoryGate::new().gate(&ledger, &history_record);

        assert!(!gate.can_dispatch_tasks);
        assert!(!gate.can_promote_side_effects);
        assert!(!gate.is_dispatchable());
        assert!(!gate.is_side_effect_safe());
        assert!(gate.requires_repair_first);
        assert_eq!(
            gate.reasons,
            vec![
                "budget_ledger_zero_budget_roles=1",
                "budget_ledger_history:budget_ledger_zero_budget_roles=1>0",
                "budget_ledger_history:budget_ledger_depleted_records=1>0",
            ]
        );
        assert_eq!(
            gate.repair_tasks
                .iter()
                .map(|task| (task.id.as_str(), task.role.clone(), task.lane.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (
                    "budget-ledger-repair-0",
                    AgentRole::Planner,
                    "budget-ledger-repair",
                ),
                (
                    "budget-ledger-repair-1",
                    AgentRole::Planner,
                    "budget-ledger-repair",
                ),
                (
                    "budget-ledger-repair-2",
                    AgentRole::Planner,
                    "budget-ledger-repair",
                ),
            ]
        );
    }

    #[test]
    fn partial_budget_depletion_gate_repairs_current_partially_depleted_roles() {
        let clean = BudgetLedgerSummary {
            roles: 1,
            zero_budget_roles: 0,
            partially_depleted_roles: 0,
            token_depleted_roles: 0,
            step_depleted_roles: 0,
            message_depleted_roles: 0,
            total_tokens: 10,
            total_steps: 1,
            total_messages: 1,
            depleted_roles: Vec::new(),
            dimension_depleted_roles: Vec::new(),
            telemetry: Vec::new(),
        };
        let ledger = BudgetLedger::new()
            .with_budget(AgentRole::Coder, AgentBudget::new(8, 0, 2))
            .with_budget(AgentRole::Reviewer, AgentBudget::new(4, 1, 1));
        let history_record = BudgetLedgerSummaryHistoryRecorder::new().record_ledger_with_health(
            BudgetLedgerSummaryHistory::from_summaries(vec![clean]),
            &ledger,
            BudgetLedgerHealthPolicy::default(),
        );

        let gate = BudgetLedgerHistoryGate::new().gate(&ledger, &history_record);

        assert!(!gate.can_dispatch_tasks);
        assert!(!gate.can_promote_side_effects);
        assert!(!gate.is_dispatchable());
        assert!(!gate.is_side_effect_safe());
        assert!(gate.requires_repair_first);
        assert_eq!(
            gate.reasons,
            vec![
                "budget_ledger_partially_depleted_roles=1",
                "budget_ledger_history:budget_ledger_partially_depleted_roles=1>0",
                "budget_ledger_history:budget_ledger_partial_depletion_records=1>0",
            ]
        );
        assert_eq!(
            gate.repair_tasks
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "budget-ledger-repair-0",
                "budget-ledger-repair-1",
                "budget-ledger-repair-2"
            ]
        );
        assert!(
            gate.telemetry.iter().any(|line| {
                line == "agent_budget_ledger_history_gate_partially_depleted_roles=1"
            })
        );
    }

    #[test]
    fn budget_ledger_history_gate_repairs_dirty_history_before_dispatch() {
        let depleted = BudgetLedgerSummary {
            roles: 1,
            zero_budget_roles: 1,
            partially_depleted_roles: 0,
            token_depleted_roles: 1,
            step_depleted_roles: 1,
            message_depleted_roles: 1,
            total_tokens: 0,
            total_steps: 0,
            total_messages: 0,
            depleted_roles: vec![AgentRole::Tester],
            dimension_depleted_roles: vec![AgentRole::Tester],
            telemetry: Vec::new(),
        };
        let ledger = BudgetLedger::new()
            .with_budget(AgentRole::Coder, AgentBudget::new(10, 1, 1))
            .with_budget(AgentRole::Reviewer, AgentBudget::new(5, 1, 1));
        let history_record = BudgetLedgerSummaryHistoryRecorder::new().record_ledger_with_health(
            BudgetLedgerSummaryHistory::from_summaries(vec![depleted]),
            &ledger,
            BudgetLedgerHealthPolicy::default(),
        );

        let gate = BudgetLedgerHistoryGate::new().gate(&ledger, &history_record);

        assert!(!gate.can_dispatch_tasks);
        assert!(!gate.can_promote_side_effects);
        assert!(gate.requires_repair_first);
        assert_eq!(gate.budget_health.status, BudgetLedgerHealthStatus::Repair);
        assert_eq!(
            gate.reasons,
            vec![
                "budget_ledger_history:budget_ledger_zero_budget_roles=1>0",
                "budget_ledger_history:budget_ledger_depleted_records=1>0",
            ]
        );
        assert_eq!(
            gate.repair_tasks
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            vec!["budget-ledger-repair-0", "budget-ledger-repair-1"]
        );
    }

    #[test]
    fn budget_ledger_history_recorder_records_and_gates_ledger() {
        let ledger = BudgetLedger::new()
            .with_budget(AgentRole::Coder, AgentBudget::new(10, 2, 2))
            .with_budget(AgentRole::Reviewer, AgentBudget::new(5, 1, 1));

        let record = BudgetLedgerSummaryHistoryRecorder::new().record_ledger_with_health_gate(
            BudgetLedgerSummaryHistory::new(),
            &ledger,
            BudgetLedgerHealthPolicy::default(),
        );

        assert_eq!(record.records(), 1);
        assert!(record.allows_service_advance());
        assert!(!record.requires_repair_first());
        assert!(record.can_dispatch_tasks());
        assert!(record.can_promote_side_effects());
        assert!(record.gate_decision.is_dispatchable());
        assert_eq!(
            record.health_record.health.status,
            BudgetLedgerHealthStatus::Stable
        );
        assert!(
            record
                .telemetry
                .iter()
                .any(|line| { line == "agent_budget_ledger_history_gate_record_dispatch=true" })
        );
    }
}
