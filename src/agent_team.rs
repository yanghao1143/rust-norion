use crate::experience::ExperienceMatch;
use crate::hardware::HardwarePlan;
use crate::hierarchy::TaskProfile;
use crate::kv_cache::MemoryMatch;
use crate::recursive_scheduler::RecursiveSchedule;
use crate::router::RouteBudget;
use crate::toolsmith::ToolsmithPlan;

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
        notes
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AgentTeamInput<'a> {
    pub prompt: &'a str,
    pub profile: TaskProfile,
    pub memories: &'a [MemoryMatch],
    pub experiences: &'a [ExperienceMatch],
    pub hardware_plan: &'a HardwarePlan,
    pub route_budget: RouteBudget,
    pub recursive_schedule: &'a RecursiveSchedule,
    pub toolsmith_plan: &'a ToolsmithPlan,
}

#[derive(Debug, Clone)]
pub struct AgentTeamPlanner {
    max_agents: usize,
    max_messages: usize,
}

impl Default for AgentTeamPlanner {
    fn default() -> Self {
        Self {
            max_agents: 7,
            max_messages: 12,
        }
    }
}

impl AgentTeamPlanner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_limits(mut self, max_agents: usize, max_messages: usize) -> Self {
        self.max_agents = max_agents.max(1);
        self.max_messages = max_messages.max(1);
        self
    }

    pub fn plan(&self, input: AgentTeamInput<'_>) -> AgentTeamPlan {
        let wants_team = contains_any(
            input.prompt,
            &[
                "agent",
                "agents",
                "subagent",
                "sub-agent",
                "orchestrator",
                "团队",
                "子agent",
                "子 agent",
                "主线程",
                "协同",
                "汇总",
                "进化",
            ],
        );

        if !wants_team {
            return AgentTeamPlan {
                main_thread_goal: compact(input.prompt, 120),
                notes: vec!["no explicit agent-team coordination requested".to_owned()],
                ..AgentTeamPlan::default()
            };
        }

        let run_id = format!(
            "agent-team-{:016x}",
            stable_hash(&format!("{:?}:{}", input.profile, input.prompt))
        );
        let mut plan = AgentTeamPlan {
            enabled: true,
            run_id: run_id.clone(),
            main_thread_goal: compact(input.prompt, 160),
            isolation: AgentIsolationPolicy {
                namespace: format!("agent_team/{run_id}"),
                ..AgentIsolationPolicy::default()
            },
            agents: default_agents(&run_id),
            messages: Vec::new(),
            conflicts: Vec::new(),
            evolution_signals: Vec::new(),
            notes: Vec::new(),
        };
        plan.agents.truncate(self.max_agents);

        push_message(
            &mut plan,
            AgentRole::Planner,
            AgentMessageKind::Task,
            "control",
            "split the request into read-only sub-agent lanes while the main thread stays the only writer",
            0.91,
            vec![format!("profile={:?}", input.profile)],
        );
        push_message(
            &mut plan,
            AgentRole::Researcher,
            AgentMessageKind::Finding,
            "context",
            "collect reusable memory and experience hints for the team blackboard",
            0.84,
            vec![
                format!("memories={}", input.memories.len()),
                format!("experiences={}", input.experiences.len()),
            ],
        );
        push_message(
            &mut plan,
            AgentRole::Coder,
            AgentMessageKind::Task,
            "implementation",
            "draft scoped implementation proposals and hand them back as structured messages",
            0.82,
            vec![format!(
                "tool_blueprints={}",
                input.toolsmith_plan.blueprint_count()
            )],
        );
        let single_writer = plan.isolation.single_writer;
        push_message(
            &mut plan,
            AgentRole::Reviewer,
            AgentMessageKind::Risk,
            "review",
            "check collisions between sub-agent proposals and the main thread state owner",
            0.88,
            vec![format!("single_writer={single_writer}")],
        );
        push_message(
            &mut plan,
            AgentRole::Tester,
            AgentMessageKind::Gate,
            "validation",
            "gate accepted changes through focused tests, trace schema, and reward notes",
            0.83,
            vec![
                format!(
                    "recursive_chunks={}",
                    input.recursive_schedule.chunk_count()
                ),
                format!(
                    "max_parallel_chunks={}",
                    input.recursive_schedule.max_parallel_chunks
                ),
            ],
        );
        push_message(
            &mut plan,
            AgentRole::Aggregator,
            AgentMessageKind::Decision,
            "blackboard",
            "aggregate conclusions, deduplicate messages, surface conflicts, and preserve evidence",
            0.90,
            vec![format!(
                "attention_fraction={:.3}",
                input.route_budget.attention_fraction
            )],
        );

        if input.experiences.is_empty() {
            push_message(
                &mut plan,
                AgentRole::MemoryCurator,
                AgentMessageKind::EvolutionHint,
                "evolution",
                "hold prompt evolution until the first evaluated experience exists",
                0.69,
                vec!["experience_replay=empty".to_owned()],
            );
        } else {
            push_message(
                &mut plan,
                AgentRole::MemoryCurator,
                AgentMessageKind::EvolutionHint,
                "evolution",
                "promote high-scoring team lessons only after process reward confirms the main result",
                0.86,
                vec![format!("experience_hints={}", input.experiences.len())],
            );
        }

        if !input.toolsmith_plan.passed_rust_gate() {
            plan.conflicts.push(AgentConflict {
                topic: "tool_surface".to_owned(),
                roles: vec![AgentRole::Coder, AgentRole::Reviewer],
                resolution: "reject non-Rust tool requests before they enter the team queue"
                    .to_owned(),
                resolved: true,
            });
        }

        if input.hardware_plan.execution.max_parallel_chunks < 2 && plan.active_agent_count() > 3 {
            plan.conflicts.push(AgentConflict {
                topic: "parallel_budget".to_owned(),
                roles: vec![AgentRole::Planner, AgentRole::Tester],
                resolution:
                    "serialize sub-agent lanes in message order under the main thread budget"
                        .to_owned(),
                resolved: true,
            });
        }

        if input.toolsmith_plan.ready_count() > 0 {
            plan.evolution_signals.push(AgentEvolutionSignal {
                target: "toolsmith_routing".to_owned(),
                action: "reuse_ready_rust_blueprints".to_owned(),
                reason:
                    "ready tool blueprints can seed future team roles without granting write access"
                        .to_owned(),
                score: (0.72 + input.toolsmith_plan.ready_count() as f32 * 0.04).min(0.94),
            });
        }
        if !input.experiences.is_empty() {
            let best_score = input
                .experiences
                .iter()
                .map(|experience| experience.score)
                .fold(0.0, f32::max);
            plan.evolution_signals.push(AgentEvolutionSignal {
                target: "team_memory_policy".to_owned(),
                action: "promote_evaluated_lessons".to_owned(),
                reason: "experience retrieval found prior rewarded lessons for the current profile"
                    .to_owned(),
                score: (0.65 + best_score * 0.25).clamp(0.0, 0.95),
            });
        }

        plan.notes.push(format!(
            "device={} pressure={:.3} collision_free={}",
            input.hardware_plan.device.as_str(),
            input.hardware_plan.pressure,
            plan.collision_free()
        ));
        plan.messages.truncate(self.max_messages);
        plan
    }
}

fn default_agents(run_id: &str) -> Vec<AgentNode> {
    [
        (
            AgentRole::Planner,
            "decompose work and assign lanes",
            "control",
            Vec::<String>::new(),
        ),
        (
            AgentRole::Researcher,
            "collect context and evidence",
            "context",
            vec!["planner".to_owned()],
        ),
        (
            AgentRole::Coder,
            "prepare implementation proposals",
            "implementation",
            vec!["planner".to_owned(), "researcher".to_owned()],
        ),
        (
            AgentRole::Reviewer,
            "detect risks and collisions",
            "review",
            vec!["coder".to_owned()],
        ),
        (
            AgentRole::Tester,
            "define verification gates",
            "validation",
            vec!["coder".to_owned(), "reviewer".to_owned()],
        ),
        (
            AgentRole::MemoryCurator,
            "extract reusable evolution signals",
            "evolution",
            vec!["aggregator".to_owned()],
        ),
        (
            AgentRole::Aggregator,
            "deduplicate and summarize blackboard messages",
            "blackboard",
            vec![
                "researcher".to_owned(),
                "reviewer".to_owned(),
                "tester".to_owned(),
            ],
        ),
    ]
    .into_iter()
    .map(|(role, objective, lane, dependencies)| AgentNode {
        id: format!("{}-{}", run_id, role.as_str()),
        role,
        objective: objective.to_owned(),
        lane: lane.to_owned(),
        dependencies,
        writes_allowed: false,
    })
    .collect()
}

fn push_message(
    plan: &mut AgentTeamPlan,
    role: AgentRole,
    kind: AgentMessageKind,
    lane: &str,
    content: &str,
    confidence: f32,
    evidence: Vec<String>,
) {
    let id = format!("{}-msg-{}", plan.run_id, plan.messages.len());
    plan.messages.push(AgentMessage {
        id,
        role,
        kind,
        lane: lane.to_owned(),
        content: content.to_owned(),
        confidence: confidence.clamp(0.0, 1.0),
        evidence,
    });
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    let lower = value.to_ascii_lowercase();
    needles.iter().any(|needle| lower.contains(needle))
}

fn stable_hash(value: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in value.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn compact(text: &str, max_chars: usize) -> String {
    let mut out = text.chars().take(max_chars).collect::<String>();
    if text.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware::HardwarePlan;
    use crate::recursive_scheduler::RecursiveSchedule;

    #[test]
    fn plans_collision_free_team_for_subagent_prompt() {
        let planner = AgentTeamPlanner::new();
        let toolsmith_plan = ToolsmithPlan::default();
        let hardware_plan = HardwarePlan::default();
        let recursive_schedule = RecursiveSchedule::default();

        let plan = planner.plan(AgentTeamInput {
            prompt: "让他拥有一个子agent团队，把消息汇总给主线程快速进化",
            profile: TaskProfile::Coding,
            memories: &[],
            experiences: &[],
            hardware_plan: &hardware_plan,
            route_budget: RouteBudget {
                threshold: 0.5,
                attention_tokens: 1,
                fast_tokens: 1,
                attention_fraction: 0.5,
            },
            recursive_schedule: &recursive_schedule,
            toolsmith_plan: &toolsmith_plan,
        });

        assert!(plan.enabled);
        assert!(plan.has_role(AgentRole::Aggregator));
        assert!(plan.collision_free());
        assert!(plan.agents.iter().all(|agent| !agent.writes_allowed));
        assert!(plan.summary().contains("collision_free=true"));
    }

    #[test]
    fn records_resolved_conflict_for_blocked_tool_surface() {
        let planner = AgentTeamPlanner::new();
        let mut toolsmith_plan = ToolsmithPlan::default();
        toolsmith_plan.rust_only = false;
        toolsmith_plan
            .rejected_requests
            .push("non_rust_tool_request_blocked".to_owned());
        let hardware_plan = HardwarePlan::default();
        let recursive_schedule = RecursiveSchedule::default();

        let plan = planner.plan(AgentTeamInput {
            prompt: "agent team should coordinate tool creation",
            profile: TaskProfile::Coding,
            memories: &[],
            experiences: &[],
            hardware_plan: &hardware_plan,
            route_budget: RouteBudget {
                threshold: 0.5,
                attention_tokens: 1,
                fast_tokens: 1,
                attention_fraction: 0.5,
            },
            recursive_schedule: &recursive_schedule,
            toolsmith_plan: &toolsmith_plan,
        });

        assert_eq!(plan.unresolved_conflict_count(), 0);
        assert!(plan.conflict_summaries(1)[0].contains("tool_surface"));
    }

    #[test]
    fn disabled_plan_has_no_reward_notes() {
        let planner = AgentTeamPlanner::new();
        let toolsmith_plan = ToolsmithPlan::default();
        let hardware_plan = HardwarePlan::default();
        let recursive_schedule = RecursiveSchedule::default();

        let plan = planner.plan(AgentTeamInput {
            prompt: "ordinary answer without delegation",
            profile: TaskProfile::General,
            memories: &[],
            experiences: &[],
            hardware_plan: &hardware_plan,
            route_budget: RouteBudget {
                threshold: 0.5,
                attention_tokens: 1,
                fast_tokens: 1,
                attention_fraction: 0.5,
            },
            recursive_schedule: &recursive_schedule,
            toolsmith_plan: &toolsmith_plan,
        });

        assert!(!plan.enabled);
        assert!(plan.reward_notes().is_empty());
    }
}
