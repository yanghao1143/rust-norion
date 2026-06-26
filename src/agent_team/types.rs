use super::util::compact;

#[derive(Debug, Clone, PartialEq)]
pub struct AgentTeamSourceSummary {
    pub source_id: String,
    pub role: String,
    pub lane: String,
    pub confidence: f32,
    pub summary: String,
}

impl AgentTeamSourceSummary {
    pub fn summary_line(&self) -> String {
        format!(
            "source={} role={} lane={} confidence={:.2} summary={}",
            self.source_id, self.role, self.lane, self.confidence, self.summary
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentTeamAggregation {
    pub lane_count: usize,
    pub message_summaries: Vec<String>,
    pub source_summaries: Vec<AgentTeamSourceSummary>,
    pub conflict_topics: Vec<String>,
    pub unresolved_conflict_topics: Vec<String>,
    pub budget_scope: String,
    pub max_parallel_lanes: usize,
    pub attention_fraction: f32,
    pub main_thread_writer: String,
}

impl Default for AgentTeamAggregation {
    fn default() -> Self {
        Self {
            lane_count: 0,
            message_summaries: Vec::new(),
            source_summaries: Vec::new(),
            conflict_topics: Vec::new(),
            unresolved_conflict_topics: Vec::new(),
            budget_scope: "disabled".to_owned(),
            max_parallel_lanes: 0,
            attention_fraction: 0.0,
            main_thread_writer: "main_thread".to_owned(),
        }
    }
}

impl AgentTeamAggregation {
    pub fn summary(&self) -> String {
        format!(
            "lanes={} summaries={} sources={} conflicts={} unresolved={} budget_scope={} max_parallel_lanes={} attention_fraction={:.3} writer={}",
            self.lane_count,
            self.message_summaries.len(),
            self.source_summaries.len(),
            self.conflict_topics.len(),
            self.unresolved_conflict_topics.len(),
            self.budget_scope,
            self.max_parallel_lanes,
            self.attention_fraction,
            self.main_thread_writer
        )
    }

    pub fn source_summary_lines(&self, limit: usize) -> Vec<String> {
        self.source_summaries
            .iter()
            .take(limit)
            .map(AgentTeamSourceSummary::summary_line)
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AgentRole {
    Planner,
    Researcher,
    Coder,
    Reviewer,
    Tester,
    MemoryCurator,
    Aggregator,
}

impl AgentRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Planner => "planner",
            Self::Researcher => "researcher",
            Self::Coder => "coder",
            Self::Reviewer => "reviewer",
            Self::Tester => "tester",
            Self::MemoryCurator => "memory_curator",
            Self::Aggregator => "aggregator",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentMessageKind {
    Task,
    Finding,
    Risk,
    Gate,
    Decision,
    EvolutionHint,
}

impl AgentMessageKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Task => "task",
            Self::Finding => "finding",
            Self::Risk => "risk",
            Self::Gate => "gate",
            Self::Decision => "decision",
            Self::EvolutionHint => "evolution_hint",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentNode {
    pub id: String,
    pub role: AgentRole,
    pub objective: String,
    pub lane: String,
    pub dependencies: Vec<String>,
    pub writes_allowed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentMessage {
    pub id: String,
    pub role: AgentRole,
    pub kind: AgentMessageKind,
    pub lane: String,
    pub content: String,
    pub confidence: f32,
    pub evidence: Vec<String>,
}

impl AgentMessage {
    pub fn summary(&self) -> String {
        format!(
            "{}:{}:{} confidence={:.2}",
            self.role.as_str(),
            self.kind.as_str(),
            compact(&self.content, 96),
            self.confidence
        )
    }
}

impl AgentTeamSourceSummary {
    pub fn from_message(message: &AgentMessage) -> Self {
        Self {
            source_id: message.id.clone(),
            role: message.role.as_str().to_owned(),
            lane: message.lane.clone(),
            confidence: message.confidence,
            summary: compact(&message.content, 96),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentConflict {
    pub topic: String,
    pub roles: Vec<AgentRole>,
    pub resolution: String,
    pub resolved: bool,
}

impl AgentConflict {
    pub fn summary(&self) -> String {
        let roles = self
            .roles
            .iter()
            .map(|role| role.as_str())
            .collect::<Vec<_>>()
            .join("+");
        format!(
            "topic={} roles={} resolved={} resolution={}",
            self.topic, roles, self.resolved, self.resolution
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentEvolutionSignal {
    pub target: String,
    pub action: String,
    pub reason: String,
    pub score: f32,
}

impl AgentEvolutionSignal {
    pub fn summary(&self) -> String {
        format!(
            "target={} action={} score={:.2} reason={}",
            self.target,
            self.action,
            self.score,
            compact(&self.reason, 80)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentIsolationPolicy {
    pub single_writer: bool,
    pub read_only_subagents: bool,
    pub namespace: String,
    pub allowed_outputs: Vec<String>,
    pub denied_capabilities: Vec<String>,
}

impl Default for AgentIsolationPolicy {
    fn default() -> Self {
        Self {
            single_writer: true,
            read_only_subagents: true,
            namespace: "agent_team/summary".to_owned(),
            allowed_outputs: vec![
                "structured_messages".to_owned(),
                "risk_notes".to_owned(),
                "validation_gates".to_owned(),
                "evolution_hints".to_owned(),
            ],
            denied_capabilities: vec![
                "direct_memory_write".to_owned(),
                "direct_adaptive_state_write".to_owned(),
                "unscoped_file_mutation".to_owned(),
                "network_side_effects".to_owned(),
            ],
        }
    }
}

impl AgentIsolationPolicy {
    pub fn collision_free(&self, agents: &[AgentNode], conflicts: &[AgentConflict]) -> bool {
        self.single_writer
            && self.read_only_subagents
            && agents.iter().all(|agent| !agent.writes_allowed)
            && conflicts.iter().all(|conflict| conflict.resolved)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentTeamPlan {
    pub enabled: bool,
    pub run_id: String,
    pub main_thread_goal: String,
    pub isolation: AgentIsolationPolicy,
    pub aggregation: AgentTeamAggregation,
    pub agents: Vec<AgentNode>,
    pub messages: Vec<AgentMessage>,
    pub conflicts: Vec<AgentConflict>,
    pub evolution_signals: Vec<AgentEvolutionSignal>,
    pub notes: Vec<String>,
}

impl Default for AgentTeamPlan {
    fn default() -> Self {
        Self {
            enabled: false,
            run_id: "agent-team-disabled".to_owned(),
            main_thread_goal: String::new(),
            isolation: AgentIsolationPolicy::default(),
            aggregation: AgentTeamAggregation::default(),
            agents: Vec::new(),
            messages: Vec::new(),
            conflicts: Vec::new(),
            evolution_signals: Vec::new(),
            notes: Vec::new(),
        }
    }
}

impl AgentTeamPlan {
    pub fn active_agent_count(&self) -> usize {
        self.agents.len()
    }

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    pub fn conflict_count(&self) -> usize {
        self.conflicts.len()
    }

    pub fn unresolved_conflict_count(&self) -> usize {
        self.conflicts
            .iter()
            .filter(|conflict| !conflict.resolved)
            .count()
    }

    pub fn evolution_signal_count(&self) -> usize {
        self.evolution_signals.len()
    }

    pub fn collision_free(&self) -> bool {
        self.isolation.collision_free(&self.agents, &self.conflicts)
    }

    pub fn has_role(&self, role: AgentRole) -> bool {
        self.agents.iter().any(|agent| agent.role == role)
    }

    pub fn summary(&self) -> String {
        format!(
            "enabled={} run_id={} agents={} messages={} conflicts={} unresolved={} evolution_signals={} collision_free={} namespace={}",
            self.enabled,
            self.run_id,
            self.active_agent_count(),
            self.message_count(),
            self.conflict_count(),
            self.unresolved_conflict_count(),
            self.evolution_signal_count(),
            self.collision_free(),
            self.isolation.namespace
        )
    }

    pub fn message_summaries(&self, limit: usize) -> Vec<String> {
        self.messages
            .iter()
            .take(limit)
            .map(AgentMessage::summary)
            .collect()
    }

    pub fn conflict_summaries(&self, limit: usize) -> Vec<String> {
        self.conflicts
            .iter()
            .take(limit)
            .map(AgentConflict::summary)
            .collect()
    }

    pub fn evolution_summaries(&self, limit: usize) -> Vec<String> {
        self.evolution_signals
            .iter()
            .take(limit)
            .map(AgentEvolutionSignal::summary)
            .collect()
    }

    pub fn reward_notes(&self) -> Vec<String> {
        if !self.enabled {
            return Vec::new();
        }

        let mut notes = vec![format!(
            "agent_team:agents={}:messages={}:conflicts={}:unresolved={}:evolution={}:collision_free={}",
            self.active_agent_count(),
            self.message_count(),
            self.conflict_count(),
            self.unresolved_conflict_count(),
            self.evolution_signal_count(),
            self.collision_free()
        )];

        notes.extend(
            self.evolution_signals
                .iter()
                .take(3)
                .map(|signal| format!("agent_team:evolve:{}:{:.2}", signal.target, signal.score)),
        );
        notes.push(format!(
            "agent_team:aggregation:lanes={}:summaries={}:budget_scope={}:max_parallel_lanes={}",
            self.aggregation.lane_count,
            self.aggregation.message_summaries.len(),
            self.aggregation.budget_scope,
            self.aggregation.max_parallel_lanes
        ));
        notes
    }
}
