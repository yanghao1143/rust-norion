use super::super::super::RuntimeRequest;
use super::super::summary::{
    bullet_list, bullet_runtime_adapter_observations, bullet_runtime_kv_blocks,
    bullet_tool_blueprints,
};
use super::json_items::task_intent_summary;

pub(in crate::runtime) fn format_runtime_prompt(request: &RuntimeRequest) -> String {
    let transformer_counts = request.transformer_plan.counts();
    let task_intent = task_intent_summary(request);
    format!(
        "Noiron runtime request\n\
         runtime: {}\n\
         runtime_architecture: {}\n\
         profile: {:?}\n\
         tenant_scope: {}\n\
         task_intent: language={} coding_language={} rust_coding={}\n\
         max_tokens: {}\n\
         route: threshold={:.3} attention_fraction={:.3} attention_tokens={} fast_tokens={}\n\
         hierarchy: global={:.3} local={:.3} convolution={:.3}\n\
         transformer: template={} global_layers={} local_layers={} convolution_layers={}\n\
         recursive: {}\n\
         hardware: {}\n\
         runtime_device_contract: {}\n\
         memory_hints:\n{}\n\
         infini_memory_hints:\n{}\n\
         experience_hints:\n{}\n\
         toolsmith: {}\n\
         tool_blueprints:\n{}\n\
         agent_team: {}\n\
         agent_team_messages:\n{}\n\
         imported_kv_blocks:\n{}\n\
         runtime_adapter_observations:\n{}\n\
         prompt:\n{}",
        request.runtime_metadata.summary(),
        request.runtime_architecture.summary(),
        request.profile,
        request.tenant_scope_summary(),
        task_intent.language_mode,
        task_intent.coding_language,
        task_intent.rust_coding,
        request.max_tokens,
        request.route_budget.threshold,
        request.route_budget.attention_fraction,
        request.route_budget.attention_tokens,
        request.route_budget.fast_tokens,
        request.hierarchy.global,
        request.hierarchy.local,
        request.hierarchy.convolution,
        request.transformer_plan.template_name(),
        transformer_counts.global,
        transformer_counts.local,
        transformer_counts.convolution,
        request.recursive_schedule.summary(),
        request.hardware_plan.summary(),
        request.hardware_plan.runtime_contract_summary(),
        bullet_list(&request.memory_hints),
        bullet_list(&request.infini_memory_hints),
        bullet_list(&request.experience_hints),
        request.toolsmith_plan.summary(),
        bullet_tool_blueprints(&request.toolsmith_plan.blueprints),
        request.agent_team_plan.summary(),
        bullet_list(&request.agent_team_plan.message_summaries(8)),
        bullet_runtime_kv_blocks(&request.imported_kv_blocks),
        bullet_runtime_adapter_observations(&request.runtime_adapter_observations),
        request.prompt
    )
}
