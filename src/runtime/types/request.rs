use crate::agent_team::AgentTeamPlan;
use crate::engine::GenerationContext;
use crate::experience::render_experience_hint;
use crate::hardware::HardwarePlan;
use crate::hierarchy::{HierarchyWeights, TaskProfile};
use crate::kv_exchange::RuntimeKvBlock;
use crate::recursive_scheduler::RecursiveSchedule;
use crate::router::RouteBudget;
use crate::runtime_manifest::TransformerRuntimeArchitecture;
use crate::tenant_scope::TenantScope;
use crate::toolsmith::ToolsmithPlan;
use crate::transformer::TransformerRefactorPlan;

use super::{RuntimeAdapterObservation, RuntimeMetadata};

#[derive(Debug, Clone)]
pub struct RuntimeRequest {
    pub prompt: String,
    pub profile: TaskProfile,
    pub tenant_scope: Option<TenantScope>,
    pub runtime_metadata: RuntimeMetadata,
    pub runtime_architecture: TransformerRuntimeArchitecture,
    pub memory_hints: Vec<String>,
    pub infini_memory_hints: Vec<String>,
    pub experience_hints: Vec<String>,
    pub runtime_adapter_observations: Vec<RuntimeAdapterObservation>,
    pub toolsmith_plan: ToolsmithPlan,
    pub agent_team_plan: AgentTeamPlan,
    pub route_budget: RouteBudget,
    pub hierarchy: HierarchyWeights,
    pub transformer_plan: TransformerRefactorPlan,
    pub recursive_schedule: RecursiveSchedule,
    pub hardware_plan: HardwarePlan,
    pub imported_kv_blocks: Vec<RuntimeKvBlock>,
    pub max_tokens: usize,
}

impl RuntimeRequest {
    pub fn from_context(
        context: &GenerationContext<'_>,
        max_tokens: usize,
        runtime_metadata: RuntimeMetadata,
        runtime_architecture: TransformerRuntimeArchitecture,
    ) -> Self {
        let runtime_adapter_observations = RuntimeAdapterObservation::from_experiences_for_hardware(
            context.experiences,
            &runtime_metadata.model_id,
            context.hardware_plan,
        );

        Self {
            prompt: context.prompt.to_owned(),
            profile: context.profile,
            tenant_scope: context.tenant_scope.cloned(),
            runtime_metadata,
            runtime_architecture,
            memory_hints: context
                .memories
                .iter()
                .map(|memory| {
                    format!(
                        "{} similarity={:.3} strength={:.3}",
                        memory.key, memory.similarity, memory.strength
                    )
                })
                .collect(),
            infini_memory_hints: context
                .infini_memory_plan
                .local_window()
                .iter()
                .chain(context.infini_memory_plan.global_memory())
                .map(|memory| {
                    format!(
                        "{:?}:{} score={:.3} tokens={} reason={}",
                        memory.scope,
                        memory.key,
                        memory.score,
                        memory.estimated_tokens,
                        memory.reason
                    )
                })
                .collect(),
            experience_hints: context
                .experiences
                .iter()
                .map(render_experience_hint)
                .collect(),
            runtime_adapter_observations,
            toolsmith_plan: context.toolsmith_plan.clone(),
            agent_team_plan: context.agent_team_plan.clone(),
            route_budget: context.route_budget,
            hierarchy: context.hierarchy,
            transformer_plan: context.transformer_plan.clone(),
            recursive_schedule: context.recursive_schedule.clone(),
            hardware_plan: context.hardware_plan.clone(),
            imported_kv_blocks: Vec::new(),
            max_tokens,
        }
    }

    pub fn with_imported_kv_blocks(mut self, blocks: Vec<RuntimeKvBlock>) -> Self {
        self.imported_kv_blocks = blocks;
        self
    }

    pub fn tenant_scope_summary(&self) -> String {
        match self.tenant_scope.as_ref() {
            Some(scope) => format!(
                "tenant={} workspace={} session={} digest={}",
                scope.tenant_id,
                scope.workspace_id,
                scope.session_id,
                scope.scope_digest()
            ),
            None => "none".to_owned(),
        }
    }
}
