use super::fields::*;

pub(super) fn evaluate_trace_agent_team(line: &str) -> Vec<String> {
    let mut failures = Vec::new();
    let Some(agent_team) = json_object_after_field(line, "agent_team") else {
        failures.push("agent_team object is missing or invalid".to_owned());
        return failures;
    };
    let Some(isolation) = json_object_after_field(agent_team, "isolation") else {
        failures.push("agent_team isolation object is missing or invalid".to_owned());
        return failures;
    };
    let Some(aggregation) = json_object_after_field(agent_team, "aggregation") else {
        failures.push("agent_team aggregation object is missing or invalid".to_owned());
        return failures;
    };
    let Some(layer_b_route) = json_object_after_field(agent_team, "layer_b_route") else {
        failures.push("agent_team layer_b_route object is missing or invalid".to_owned());
        return failures;
    };

    let enabled = extract_json_bool_field(agent_team, "enabled").unwrap_or(false);
    let layer_b_route_proof_ready =
        extract_json_bool_field(agent_team, "layer_b_route_proof_ready").unwrap_or(false);
    let agents = extract_json_usize_field(agent_team, "agents").unwrap_or(0);
    let messages = extract_json_usize_field(agent_team, "messages").unwrap_or(0);
    let unresolved_conflicts =
        extract_json_usize_field(agent_team, "unresolved_conflicts").unwrap_or(0);
    let collision_free = extract_json_bool_field(agent_team, "collision_free").unwrap_or(false);
    let single_writer = extract_json_bool_field(isolation, "single_writer");
    let read_only_subagents = extract_json_bool_field(isolation, "read_only_subagents");
    let lane_count = extract_json_usize_field(aggregation, "lane_count").unwrap_or(0);
    let aggregation_messages =
        extract_json_string_array_field(aggregation, "message_summaries").unwrap_or_default();
    let unresolved_topics =
        extract_json_string_array_field(aggregation, "unresolved_conflict_topics")
            .unwrap_or_default();
    let budget_scope = extract_json_string_field(aggregation, "budget_scope").unwrap_or_default();
    let max_parallel_lanes =
        extract_json_usize_field(aggregation, "max_parallel_lanes").unwrap_or(0);
    let attention_fraction =
        extract_json_f32_field(aggregation, "attention_fraction").unwrap_or(f32::NAN);
    let main_thread_writer =
        extract_json_string_field(aggregation, "main_thread_writer").unwrap_or_default();

    if single_writer != Some(true) {
        failures.push("agent_team isolation single_writer must be true".to_owned());
    }
    if read_only_subagents != Some(true) {
        failures.push("agent_team isolation read_only_subagents must be true".to_owned());
    }
    if collision_free && unresolved_conflicts > 0 {
        failures.push(format!(
            "agent_team collision_free=true with unresolved_conflicts={unresolved_conflicts}"
        ));
    }
    if unresolved_topics.len() != unresolved_conflicts {
        failures.push(format!(
            "agent_team aggregation unresolved topics {} do not match unresolved_conflicts {unresolved_conflicts}",
            unresolved_topics.len()
        ));
    }
    if enabled {
        if !layer_b_route_proof_ready {
            failures.push("agent_team enabled=true requires Layer B route proof".to_owned());
        }
        for field in [
            "model_registry_id",
            "model_profile_id",
            "inference_backend_id",
            "model_pool_id",
        ] {
            if extract_json_string_field(layer_b_route, field)
                .is_none_or(|value| value.trim().is_empty())
            {
                failures.push(format!("agent_team Layer B route missing {field}"));
            }
        }
        if agents == 0 {
            failures.push("agent_team enabled=true requires at least one agent".to_owned());
        }
        if lane_count == 0 {
            failures.push("agent_team enabled=true requires aggregated lanes".to_owned());
        }
        if aggregation_messages.len() > messages {
            failures.push(format!(
                "agent_team aggregation summaries {} exceed messages {messages}",
                aggregation_messages.len()
            ));
        }
        if max_parallel_lanes == 0 || max_parallel_lanes > agents {
            failures.push(format!(
                "agent_team max_parallel_lanes {max_parallel_lanes} must be within 1..={agents}"
            ));
        }
    } else if lane_count != 0 || max_parallel_lanes != 0 {
        failures.push("agent_team disabled plan must not reserve lanes".to_owned());
    }
    if !matches!(
        budget_scope.as_str(),
        "disabled"
            | "serialized_read_only_lanes_under_main_thread"
            | "parallel_read_only_lanes_under_main_thread"
    ) {
        failures.push(format!("agent_team budget_scope={budget_scope} is invalid"));
    }
    if !(0.0..=1.0).contains(&attention_fraction) {
        failures.push(format!(
            "agent_team attention_fraction {attention_fraction:.6} must stay within 0.0..=1.0"
        ));
    }
    if main_thread_writer != "main_thread" {
        failures.push(format!(
            "agent_team main_thread_writer={main_thread_writer} must remain main_thread"
        ));
    }

    failures
}
