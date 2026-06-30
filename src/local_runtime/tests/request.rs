use crate::hierarchy::TaskProfile;
use crate::runtime::{RuntimeMetadata, RuntimeRequest};
use crate::runtime_manifest::TransformerRuntimeArchitecture;

pub(super) fn runtime_request(
    prompt: impl Into<String>,
    profile: TaskProfile,
    runtime_metadata: RuntimeMetadata,
    runtime_architecture: TransformerRuntimeArchitecture,
) -> RuntimeRequest {
    let route_budget = crate::router::RouteBudget {
        threshold: 0.5,
        attention_tokens: 2,
        fast_tokens: 1,
        attention_fraction: 0.66,
    };
    let hierarchy = crate::hierarchy::HierarchyWeights::new(0.2, 0.6, 0.2);

    RuntimeRequest {
        prompt: prompt.into(),
        profile,
        tenant_scope: None,
        runtime_metadata,
        runtime_architecture,
        memory_hints: Vec::new(),
        infini_memory_hints: Vec::new(),
        experience_hints: Vec::new(),
        runtime_adapter_observations: Vec::new(),
        toolsmith_plan: crate::toolsmith::ToolsmithPlan::default(),
        agent_team_plan: crate::agent_team::AgentTeamPlan::default(),
        route_budget,
        hierarchy,
        transformer_plan: crate::transformer::TransformerPlanner::new(6, 128).plan(
            profile,
            hierarchy,
            route_budget,
        ),
        recursive_schedule: crate::recursive_scheduler::RecursiveSchedule::default(),
        hardware_plan: crate::hardware::HardwarePlan::default(),
        imported_kv_blocks: Vec::new(),
        max_tokens: 64,
    }
}
