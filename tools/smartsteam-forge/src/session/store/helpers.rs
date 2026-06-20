use std::time::{SystemTime, UNIX_EPOCH};

use super::SessionRecord;

pub(in crate::session::store) fn select_record_error(
    selector: &str,
    not_found: bool,
) -> Result<SessionRecord, String> {
    if not_found {
        Err(format!("session not found: {selector}"))
    } else {
        Err(format!("session selector is ambiguous: {selector}"))
    }
}

pub(in crate::session::store) fn short_preview(value: &str) -> String {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut preview = normalized.chars().take(80).collect::<String>();
    if normalized.chars().count() > 80 {
        preview.push_str("...");
    }
    preview
}

pub(in crate::session::store) fn gate_outcome(report: &str) -> Option<&'static str> {
    report.lines().find_map(|line| {
        let line = line.trim().to_ascii_lowercase();
        if !line.starts_with("overall:") {
            return None;
        }
        if line.contains("pass") {
            Some("PASS")
        } else if line.contains("fail") {
            Some("FAIL")
        } else {
            None
        }
    })
}

pub(in crate::session::store) fn is_health_check_kind(kind: &str) -> bool {
    matches!(kind, "health_check" | "health_check_error")
}

pub(in crate::session::store) fn is_preflight_kind(kind: &str) -> bool {
    matches!(kind, "preflight" | "preflight_error")
}

pub(in crate::session::store) fn is_diagnostic_kind(kind: &str) -> bool {
    kind == "diagnostic_report"
}

pub(in crate::session::store) fn unix_timestamp_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub(in crate::session::store) fn unix_timestamp_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
