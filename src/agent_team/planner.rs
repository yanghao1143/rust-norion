use crate::experience::ExperienceMatch;
use crate::hardware::HardwarePlan;
use crate::hierarchy::TaskProfile;
use crate::kv_cache::MemoryMatch;
use crate::recursive_scheduler::RecursiveSchedule;
use crate::router::RouteBudget;
use crate::toolsmith::ToolsmithPlan;

use super::types::{
    AgentConflict, AgentEvolutionSignal, AgentIsolationPolicy, AgentMessage, AgentMessageKind,
    AgentNode, AgentRole, AgentTeamPlan,
};
use super::util::{compact, contains_any, stable_hash};

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
