use super::status_json::{
    bool_value_text, json_bool_field, json_object_field, json_string_array_field,
    json_string_literal, scalar_value,
};

pub(super) fn daemon_log_tail_status_line(daemon: &str, ledger: Option<&str>) -> Option<String> {
    let status = DaemonLogTailStatus::from_status(daemon, ledger);
    if !status.stale_when_not_running || status.latest_stdout_round.is_none() {
        return None;
    }

    Some(format!(
        "daemon_log_tail latest_stdout_round={} ledger_latest_round={} stale_when_not_running=true round_ahead_of_ledger={} note=stdout_tail_is_history_not_active_process",
        status.latest_stdout_round_line_value(),
        status.ledger_latest_round_line_value(),
        bool_value_text(status.round_ahead_of_ledger)
    ))
}

pub(super) fn daemon_log_tail_status_json(daemon: &str, ledger: Option<&str>) -> String {
    let status = DaemonLogTailStatus::from_status(daemon, ledger);
    format!(
        "{{\"read_only\":true,\"starts_process\":false,\"sends_prompt\":false,\"latest_stdout_round\":{},\"ledger_latest_round\":{},\"stale_when_not_running\":{},\"round_ahead_of_ledger\":{},\"note\":{}}}",
        optional_u64_json(status.latest_stdout_round),
        optional_u64_json(status.ledger_latest_round),
        bool_value_text(status.stale_when_not_running),
        bool_value_text(status.round_ahead_of_ledger),
        status.note_json()
    )
}

struct DaemonLogTailStatus {
    latest_stdout_round: Option<u64>,
    ledger_latest_round: Option<u64>,
    stale_when_not_running: bool,
    round_ahead_of_ledger: bool,
}

impl DaemonLogTailStatus {
    fn from_status(daemon: &str, ledger: Option<&str>) -> Self {
        let latest_stdout_round = json_string_array_field(daemon, "stdout_tail")
            .unwrap_or_default()
            .iter()
            .filter_map(|line| round_marker(line))
            .max();
        let ledger_latest_round = ledger.and_then(ledger_latest_round);
        let daemon_running = json_bool_field(daemon, "running").unwrap_or(false);
        let stale_when_not_running = latest_stdout_round.is_some() && !daemon_running;
        let round_ahead_of_ledger = latest_stdout_round
            .zip(ledger_latest_round)
            .is_some_and(|(stdout_round, ledger_round)| stdout_round > ledger_round);

        Self {
            latest_stdout_round,
            ledger_latest_round,
            stale_when_not_running,
            round_ahead_of_ledger,
        }
    }

    fn latest_stdout_round_line_value(&self) -> String {
        self.latest_stdout_round
            .map(|round| round.to_string())
            .unwrap_or_else(|| "unknown".to_owned())
    }

    fn ledger_latest_round_line_value(&self) -> String {
        self.ledger_latest_round
            .map(|round| round.to_string())
            .unwrap_or_else(|| "unknown".to_owned())
    }

    fn note_json(&self) -> String {
        if self.stale_when_not_running {
            json_string_literal("stdout_tail_is_history_not_active_process")
        } else {
            "null".to_owned()
        }
    }
}

fn ledger_latest_round(ledger: &str) -> Option<u64> {
    let latest = json_object_field(ledger, "latest")?;
    scalar_value(latest, "round").parse::<u64>().ok()
}

fn round_marker(value: &str) -> Option<u64> {
    let marker = "[round ";
    let start = value.find(marker)? + marker.len();
    let digits = value
        .get(start..)?
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>();
    if digits.is_empty() {
        return None;
    }
    digits.parse::<u64>().ok()
}

fn optional_u64_json(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_not_running_stdout_tail_as_history() {
        let daemon = r#"{
            "running": false,
            "stdout_tail": ["[round 36] stage gates:done", "[round 37] done [DONE]"]
        }"#;
        let ledger = r#"{"latest": {"round": 36}}"#;

        let line = daemon_log_tail_status_line(daemon, Some(ledger)).unwrap();
        let json = daemon_log_tail_status_json(daemon, Some(ledger));

        assert!(line.contains("latest_stdout_round=37"));
        assert!(line.contains("ledger_latest_round=36"));
        assert!(line.contains("stale_when_not_running=true"));
        assert!(line.contains("round_ahead_of_ledger=true"));
        assert!(json.contains("\"latest_stdout_round\":37"));
        assert!(json.contains("\"ledger_latest_round\":36"));
        assert!(json.contains("\"stale_when_not_running\":true"));
        assert!(json.contains("\"round_ahead_of_ledger\":true"));
        assert!(json.contains("\"note\":\"stdout_tail_is_history_not_active_process\""));
    }

    #[test]
    fn does_not_mark_running_daemon_tail_as_stale() {
        let daemon = r#"{"running": true, "stdout_tail": ["[round 4] working"]}"#;
        let ledger = r#"{"latest": {"round": 4}}"#;

        assert!(daemon_log_tail_status_line(daemon, Some(ledger)).is_none());
        let json = daemon_log_tail_status_json(daemon, Some(ledger));

        assert!(json.contains("\"latest_stdout_round\":4"));
        assert!(json.contains("\"stale_when_not_running\":false"));
        assert!(json.contains("\"round_ahead_of_ledger\":false"));
        assert!(json.contains("\"note\":null"));
    }
}
