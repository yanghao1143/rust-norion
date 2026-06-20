use crate::agent_team::AgentTeamPlan;
use crate::hierarchy::{HierarchyWeights, TaskProfile};
use crate::infini_memory::InfiniMemoryCounts;
use crate::recursive_scheduler::RecursiveSchedule;
use crate::router::{GenerationMetrics, RouteBudget};
use crate::tiered_cache::TierCounts;
use crate::toolsmith::ToolsmithPlan;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RewardAction {
    Reinforce,
    Hold,
    Penalize,
}

impl RewardAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Reinforce => "reinforce",
            Self::Hold => "hold",
            Self::Penalize => "penalize",
        }
    }
}

impl std::str::FromStr for RewardAction {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "reinforce" => Ok(Self::Reinforce),
            "hold" => Ok(Self::Hold),
            "penalize" => Ok(Self::Penalize),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ProcessRewardComponents {
    pub route: f32,
    pub memory: f32,
    pub hierarchy: f32,
    pub reflection: f32,
    pub latency: f32,
    pub admission: f32,
}

impl Default for ProcessRewardComponents {
    fn default() -> Self {
        Self {
            route: 0.5,
            memory: 0.5,
            hierarchy: 0.5,
            reflection: 0.5,
            latency: 0.5,
            admission: 0.5,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProcessRewardReport {
    pub total: f32,
    pub components: ProcessRewardComponents,
    pub action: RewardAction,
    pub notes: Vec<String>,
}

impl Default for ProcessRewardReport {
    fn default() -> Self {
        Self {
            total: 0.5,
            components: ProcessRewardComponents::default(),
            action: RewardAction::Hold,
            notes: Vec::new(),
        }
    }
}

impl ProcessRewardReport {
    pub fn summary(&self) -> String {
        format!(
            "total={:.3} action={} route={:.3} memory={:.3} hierarchy={:.3} reflection={:.3} latency={:.3} admission={:.3}",
            self.total,
            self.action.as_str(),
            self.components.route,
            self.components.memory,
            self.components.hierarchy,
            self.components.reflection,
            self.components.latency,
            self.components.admission
        )
    }
}

#[derive(Debug, Clone)]
pub struct ProcessRewardInput {
    pub profile: TaskProfile,
    pub route_budget: RouteBudget,
    pub hierarchy: HierarchyWeights,
    pub metrics: GenerationMetrics,
    pub quality: f32,
    pub contradiction_count: usize,
    pub reflection_issue_count: usize,
    pub critical_reflection_issue_count: usize,
    pub revision_action_count: usize,
    pub used_memories: usize,
    pub used_experiences: usize,
    pub tier_counts: TierCounts,
    pub infini_counts: InfiniMemoryCounts,
    pub recursive_schedule: RecursiveSchedule,
    pub recursive_runtime_calls: usize,
    pub stream_windows: usize,
    pub stored_memory: bool,
    pub stored_gist_memories: usize,
    pub stored_runtime_kv_memories: usize,
    pub gist_records: usize,
    pub toolsmith_plan: ToolsmithPlan,
    pub agent_team_plan: AgentTeamPlan,
}
