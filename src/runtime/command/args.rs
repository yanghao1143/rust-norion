use crate::toolsmith::ToolBlueprint;

use super::CommandWireFormat;
use crate::runtime::{RuntimeAdapterObservation, RuntimeRequest, runtime_kv_blocks_summary};

pub(in crate::runtime::command) fn expand_command_arg(
    arg: &str,
    request: &RuntimeRequest,
    payload: &str,
    wire_format: CommandWireFormat,
) -> String {
    let mut expanded = String::with_capacity(arg.len());
    let mut index = 0;
    while index < arg.len() {
        let tail = &arg[index..];
        if tail.starts_with('{')
            && let Some((placeholder, value)) =
                command_arg_placeholder_value(tail, request, payload, wire_format)
        {
            expanded.push_str(&value);
            index += placeholder.len();
            continue;
        }

        let Some(ch) = tail.chars().next() else {
            break;
        };
        expanded.push(ch);
        index += ch.len_utf8();
    }
    expanded
}

fn command_arg_placeholder_value(
    input: &str,
    request: &RuntimeRequest,
    payload: &str,
    wire_format: CommandWireFormat,
) -> Option<(&'static str, String)> {
    if input.starts_with("{prompt}") {
        return Some(("{prompt}", payload.to_owned()));
    }
    if input.starts_with("{runtime_payload}") {
        return Some(("{runtime_payload}", payload.to_owned()));
    }
    if input.starts_with("{user_prompt}") {
        return Some(("{user_prompt}", request.prompt.clone()));
    }
    if input.starts_with("{task_prompt}") {
        return Some(("{task_prompt}", request.prompt.clone()));
    }
    if input.starts_with("{wire_format}") {
        return Some(("{wire_format}", wire_format.as_str().to_owned()));
    }
    if input.starts_with("{max_tokens}") {
        return Some(("{max_tokens}", request.max_tokens.to_string()));
    }
    if input.starts_with("{tenant_scope}") {
        return Some(("{tenant_scope}", request.tenant_scope_summary()));
    }
    if input.starts_with("{tenant_id}") {
        return Some((
            "{tenant_id}",
            request
                .tenant_scope
                .as_ref()
                .map(|scope| scope.tenant_id.clone())
                .unwrap_or_default(),
        ));
    }
    if input.starts_with("{workspace_id}") {
        return Some((
            "{workspace_id}",
            request
                .tenant_scope
                .as_ref()
                .map(|scope| scope.workspace_id.clone())
                .unwrap_or_default(),
        ));
    }
    if input.starts_with("{session_id}") {
        return Some((
            "{session_id}",
            request
                .tenant_scope
                .as_ref()
                .map(|scope| scope.session_id.clone())
                .unwrap_or_default(),
        ));
    }
    if input.starts_with("{memory_hints}") {
        return Some(("{memory_hints}", request.memory_hints.join("\n")));
    }
    if input.starts_with("{infini_memory_hints}") {
        return Some((
            "{infini_memory_hints}",
            request.infini_memory_hints.join("\n"),
        ));
    }
    if input.starts_with("{experience_hints}") {
        return Some(("{experience_hints}", request.experience_hints.join("\n")));
    }
    if input.starts_with("{tool_blueprints}") {
        return Some((
            "{tool_blueprints}",
            request
                .toolsmith_plan
                .blueprints
                .iter()
                .map(ToolBlueprint::summary)
                .collect::<Vec<_>>()
                .join("\n"),
        ));
    }
    if input.starts_with("{toolsmith_plan}") {
        return Some(("{toolsmith_plan}", request.toolsmith_plan.summary()));
    }
    if input.starts_with("{agent_team_plan}") {
        return Some(("{agent_team_plan}", request.agent_team_plan.summary()));
    }
    if input.starts_with("{agent_team_messages}") {
        return Some((
            "{agent_team_messages}",
            request.agent_team_plan.message_summaries(8).join("\n"),
        ));
    }
    if input.starts_with("{imported_kv_blocks}") {
        return Some((
            "{imported_kv_blocks}",
            runtime_kv_blocks_summary(&request.imported_kv_blocks),
        ));
    }
    if input.starts_with("{runtime_adapter_observations}") {
        return Some((
            "{runtime_adapter_observations}",
            request
                .runtime_adapter_observations
                .iter()
                .map(RuntimeAdapterObservation::summary)
                .collect::<Vec<_>>()
                .join("\n"),
        ));
    }
    if input.starts_with("{recursive_schedule}") {
        return Some(("{recursive_schedule}", request.recursive_schedule.summary()));
    }
    if input.starts_with("{runtime_device_contract}") {
        return Some((
            "{runtime_device_contract}",
            request.hardware_plan.runtime_contract_summary(),
        ));
    }
    if input.starts_with("{runtime_metadata}") {
        return Some(("{runtime_metadata}", request.runtime_metadata.summary()));
    }
    if input.starts_with("{runtime_architecture}") {
        return Some((
            "{runtime_architecture}",
            request.runtime_architecture.summary(),
        ));
    }
    None
}
