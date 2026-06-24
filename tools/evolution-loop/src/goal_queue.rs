use crate::json::{json_string, json_string_array, preview_text};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GoalKind {
    Repair,
    Relabel,
    Splice,
    Fallback,
}

impl GoalKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Repair => "repair",
            Self::Relabel => "relabel",
            Self::Splice => "splice",
            Self::Fallback => "fallback",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GoalQueueInput<'a> {
    pub(crate) source: &'a str,
    pub(crate) latest_round: Option<u64>,
    pub(crate) latest_success: Option<bool>,
    pub(crate) latest_error: Option<&'a str>,
    pub(crate) stream_truncation_failures: usize,
    pub(crate) missing_final_failures: usize,
    pub(crate) runtime_response_failures: usize,
    pub(crate) recent_stream_truncation_failures: usize,
    pub(crate) recent_missing_final_failures: usize,
    pub(crate) recent_runtime_response_failures: usize,
    pub(crate) gate_failures: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GoalItem {
    goal_id: String,
    kind: GoalKind,
    trigger: String,
    priority: u8,
    source_round: Option<u64>,
    action: String,
    reason: String,
    ready_for_next_round: bool,
    releases_repair_factor: bool,
    relabel_required: bool,
}

pub(crate) fn ledger_goal_queue_json(input: GoalQueueInput<'_>) -> String {
    let goals = ledger_goal_queue(&input);
    goal_queue_json("evolution_goal_queue_v1", input.source, &goals)
}

fn ledger_goal_queue(input: &GoalQueueInput<'_>) -> Vec<GoalItem> {
    let mut goals = Vec::new();
    if input.latest_success == Some(false) {
        goals.push(goal_item(
            "latest_failure",
            GoalKind::Repair,
            "failure",
            100,
            input.latest_round,
            "repair_latest_failed_round",
            input.latest_error.unwrap_or("latest round failed"),
        ));
    }
    if input.recent_stream_truncation_failures > 0 || input.stream_truncation_failures > 0 {
        goals.push(goal_item(
            "stream_truncation",
            GoalKind::Splice,
            "failure",
            90,
            input.latest_round,
            "splice_stream_terminal_evidence",
            "stream truncated before terminal event",
        ));
    }
    if input.recent_missing_final_failures > 0 || input.missing_final_failures > 0 {
        goals.push(goal_item(
            "missing_final",
            GoalKind::Splice,
            "failure",
            88,
            input.latest_round,
            "splice_missing_final_event_to_ledger_repair",
            "stream ended without final event",
        ));
    }
    if input.recent_runtime_response_failures > 0 || input.runtime_response_failures > 0 {
        goals.push(goal_item(
            "runtime_response",
            GoalKind::Fallback,
            "failure",
            82,
            input.latest_round,
            "fallback_runtime_or_pool_route",
            "runtime response failed or returned unusable evidence",
        ));
    }
    if let Some(reason) = relabel_reason(input.gate_failures) {
        goals.push(goal_item(
            "gate_relabel",
            GoalKind::Relabel,
            "failure",
            76,
            input.latest_round,
            "relabel_failed_gate_to_repair_target",
            reason,
        ));
    }
    if !input.gate_failures.is_empty() && !goals.iter().any(|goal| goal.kind == GoalKind::Repair) {
        goals.push(goal_item(
            "gate_repair",
            GoalKind::Repair,
            "failure",
            72,
            input.latest_round,
            "repair_report_gate_blocker",
            input
                .gate_failures
                .first()
                .map(String::as_str)
                .unwrap_or("report gate failed"),
        ));
    }
    goals
}

fn relabel_reason(gate_failures: &[String]) -> Option<&str> {
    gate_failures
        .iter()
        .find(|failure| {
            let lower = failure.to_ascii_lowercase();
            lower.contains("helper")
                || lower.contains("test-gate")
                || lower.contains("validation")
                || lower.contains("label")
                || lower.contains("role")
        })
        .map(String::as_str)
}

fn goal_item(
    id_suffix: &str,
    kind: GoalKind,
    trigger: &str,
    priority: u8,
    source_round: Option<u64>,
    action: &str,
    reason: &str,
) -> GoalItem {
    let round = source_round
        .map(|round| format!("r{round}"))
        .unwrap_or_else(|| "unknown".to_owned());
    GoalItem {
        goal_id: format!("evolution-goal-{round}-{id_suffix}"),
        kind,
        trigger: trigger.to_owned(),
        priority,
        source_round,
        action: action.to_owned(),
        reason: preview_text(reason, 220),
        ready_for_next_round: true,
        releases_repair_factor: matches!(kind, GoalKind::Repair | GoalKind::Splice),
        relabel_required: kind == GoalKind::Relabel,
    }
}

fn goal_queue_json(schema: &str, source: &str, goals: &[GoalItem]) -> String {
    let kinds = goals
        .iter()
        .map(|goal| goal.kind.as_str().to_owned())
        .collect::<Vec<_>>();
    let items = goals.iter().map(goal_json).collect::<Vec<_>>().join(",");
    format!(
        "{{\"schema\":{},\"source\":{},\"read_only\":true,\"report_only\":true,\"side_effects\":false,\"starts_process\":false,\"sends_prompt\":false,\"queue_len\":{},\"executable_goal_count\":{},\"goal_kinds\":{},\"goals\":[{}]}}",
        json_string(schema),
        json_string(source),
        goals.len(),
        goals
            .iter()
            .filter(|goal| goal.ready_for_next_round)
            .count(),
        json_string_array(&kinds),
        items
    )
}

fn goal_json(goal: &GoalItem) -> String {
    format!(
        "{{\"goal_id\":{},\"kind\":{},\"trigger\":{},\"priority\":{},\"source_round\":{},\"action\":{},\"reason\":{},\"ready_for_next_round\":{},\"releases_repair_factor\":{},\"relabel_required\":{}}}",
        json_string(&goal.goal_id),
        json_string(goal.kind.as_str()),
        json_string(&goal.trigger),
        goal.priority,
        goal.source_round
            .map(|round| round.to_string())
            .unwrap_or_else(|| "null".to_owned()),
        json_string(&goal.action),
        json_string(&goal.reason),
        goal.ready_for_next_round,
        goal.releases_repair_factor,
        goal.relabel_required
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ledger_queue_classifies_failure_repair_splice_fallback_and_relabel() {
        let gate_failures = vec![
            "latest helper stage feedback for review missing required fields".to_owned(),
            "runtime response failures 1 above maximum 0".to_owned(),
        ];
        let json = ledger_goal_queue_json(GoalQueueInput {
            source: "ledger_report",
            latest_round: Some(7),
            latest_success: Some(false),
            latest_error: Some("stream ended without final event"),
            stream_truncation_failures: 1,
            missing_final_failures: 1,
            runtime_response_failures: 1,
            recent_stream_truncation_failures: 1,
            recent_missing_final_failures: 1,
            recent_runtime_response_failures: 1,
            gate_failures: &gate_failures,
        });

        assert!(json.contains("\"schema\":\"evolution_goal_queue_v1\""));
        assert!(json.contains("\"kind\":\"repair\""));
        assert!(json.contains("\"kind\":\"splice\""));
        assert!(json.contains("\"kind\":\"fallback\""));
        assert!(json.contains("\"kind\":\"relabel\""));
        assert!(json.contains("\"releases_repair_factor\":true"));
        assert!(json.contains("\"relabel_required\":true"));
        assert!(json.contains("\"source_round\":7"));
    }

    #[test]
    fn ledger_queue_stays_empty_for_clean_successful_ledger() {
        let json = ledger_goal_queue_json(GoalQueueInput {
            source: "ledger_report",
            latest_round: Some(2),
            latest_success: Some(true),
            latest_error: None,
            stream_truncation_failures: 0,
            missing_final_failures: 0,
            runtime_response_failures: 0,
            recent_stream_truncation_failures: 0,
            recent_missing_final_failures: 0,
            recent_runtime_response_failures: 0,
            gate_failures: &[],
        });

        assert!(json.contains("\"queue_len\":0"));
        assert!(json.contains("\"goals\":[]"));
    }
}
