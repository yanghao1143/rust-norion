use super::FinalPayloadSummary;

impl FinalPayloadSummary {
    pub fn status_line(&self) -> String {
        let mut parts = vec!["final".to_owned()];
        push_bool(&mut parts, "ok", self.ok);
        push_bool(&mut parts, "passed", self.passed);
        push_value(&mut parts, "runtime_model", self.runtime_model.as_deref());
        push_value(
            &mut parts,
            "runtime_tokens",
            self.runtime_token_count.as_deref(),
        );
        push_bool(
            &mut parts,
            "runtime_uncertainty",
            self.runtime_uncertainty_signal,
        );
        push_value(
            &mut parts,
            "device",
            self.runtime_device_execution_source.as_deref(),
        );
        push_value(
            &mut parts,
            "pool_role",
            self.pool_dispatch_selected_role.as_deref(),
        );
        push_bool(
            &mut parts,
            "pool_forwarded",
            self.pool_dispatch_worker_forwarded,
        );
        push_value(&mut parts, "pool_mode", self.pool_dispatch_mode.as_deref());
        push_value(
            &mut parts,
            "pool_reason",
            self.pool_dispatch_reason.as_deref(),
        );
        push_bool(&mut parts, "generate_passed", self.generate_passed);
        push_bool(&mut parts, "feedback_passed", self.feedback_passed);
        push_bool(&mut parts, "feedback_applied", self.feedback_applied);
        push_value(
            &mut parts,
            "feedback_applied_count",
            self.feedback_applied_count.as_deref(),
        );
        push_bool(&mut parts, "rust_check_checked", self.rust_check_checked);
        push_bool(&mut parts, "rust_check_passed", self.rust_check_passed);
        push_value(
            &mut parts,
            "rust_check_feedback_applied",
            self.rust_check_feedback_applied_count.as_deref(),
        );
        push_bool(
            &mut parts,
            "self_improve_checked",
            self.self_improve_checked,
        );
        push_bool(&mut parts, "self_improve_passed", self.self_improve_passed);
        push_bool(&mut parts, "state_gate_checked", self.state_gate_checked);
        push_bool(&mut parts, "state_gate_passed", self.state_gate_passed);
        push_bool(&mut parts, "trace_gate_checked", self.trace_gate_checked);
        push_bool(&mut parts, "trace_gate_passed", self.trace_gate_passed);
        if let Some(error) = self.error.as_deref().filter(|error| !error.is_empty()) {
            parts.push(format!("error=\"{}\"", short_preview(error, 120)));
        }
        if let Some(answer) = self.answer.as_deref().filter(|answer| !answer.is_empty()) {
            parts.push(format!("answer=\"{}\"", short_preview(answer, 120)));
        }
        parts.join(" ")
    }

    pub fn gate_report(&self) -> Option<String> {
        let has_gate_signal = [
            self.passed,
            self.generate_passed,
            self.feedback_passed,
            self.feedback_applied,
            self.feedback_applied_count.as_ref().map(|_| true),
            self.rust_check_checked,
            self.rust_check_passed,
            self.rust_check_feedback_applied_count
                .as_ref()
                .map(|_| true),
            self.self_improve_checked,
            self.self_improve_passed,
            self.state_gate_checked,
            self.state_gate_passed,
            self.trace_gate_checked,
            self.trace_gate_passed,
        ]
        .iter()
        .any(Option::is_some);
        if !has_gate_signal {
            return None;
        }

        let mut lines = vec!["Business-cycle gate report".to_owned()];
        push_gate_line(&mut lines, "overall", self.passed);
        push_gate_line(&mut lines, "generate", self.generate_passed);
        push_gate_line(&mut lines, "feedback", self.feedback_passed);
        push_applied_line(
            &mut lines,
            "feedback applied",
            self.feedback_applied,
            self.feedback_applied_count.as_deref(),
        );
        push_rust_check_line(&mut lines, self.rust_check_checked, self.rust_check_passed);
        push_count_line(
            &mut lines,
            "rust check feedback applied",
            self.rust_check_feedback_applied_count.as_deref(),
        );
        push_checked_gate_line(
            &mut lines,
            "self improve",
            self.self_improve_checked,
            self.self_improve_passed,
        );
        push_checked_gate_line(
            &mut lines,
            "state gate",
            self.state_gate_checked,
            self.state_gate_passed,
        );
        push_checked_gate_line(
            &mut lines,
            "trace gate",
            self.trace_gate_checked,
            self.trace_gate_passed,
        );

        let mut runtime = Vec::new();
        push_runtime_value(&mut runtime, "model", self.runtime_model.as_deref());
        push_runtime_value(&mut runtime, "tokens", self.runtime_token_count.as_deref());
        if let Some(uncertainty) = self.runtime_uncertainty_signal {
            runtime.push(format!("uncertainty={uncertainty}"));
        }
        push_runtime_value(
            &mut runtime,
            "device",
            self.runtime_device_execution_source.as_deref(),
        );
        if !runtime.is_empty() {
            lines.push(format!("runtime: {}", runtime.join(" ")));
        }
        let mut pool_dispatch = Vec::new();
        push_runtime_value(
            &mut pool_dispatch,
            "role",
            self.pool_dispatch_selected_role.as_deref(),
        );
        if let Some(worker_forwarded) = self.pool_dispatch_worker_forwarded {
            pool_dispatch.push(format!("forwarded={worker_forwarded}"));
        }
        push_runtime_value(
            &mut pool_dispatch,
            "mode",
            self.pool_dispatch_mode.as_deref(),
        );
        push_runtime_value(
            &mut pool_dispatch,
            "reason",
            self.pool_dispatch_reason.as_deref(),
        );
        if !pool_dispatch.is_empty() {
            lines.push(format!("pool dispatch: {}", pool_dispatch.join(" ")));
        }

        if let Some(answer) = self.answer.as_deref().filter(|answer| !answer.is_empty()) {
            lines.push(format!("answer: {}", short_preview(answer, 160)));
        }
        if let Some(error) = self.error.as_deref().filter(|error| !error.is_empty()) {
            lines.push(format!("error: {}", short_preview(error, 160)));
        }

        Some(lines.join("\n"))
    }
}

fn push_bool(parts: &mut Vec<String>, name: &str, value: Option<bool>) {
    if let Some(value) = value {
        parts.push(format!("{name}={value}"));
    }
}

fn push_value(parts: &mut Vec<String>, name: &str, value: Option<&str>) {
    if let Some(value) = value.filter(|value| !value.is_empty() && *value != "null") {
        parts.push(format!("{name}={value}"));
    }
}

fn push_gate_line(lines: &mut Vec<String>, name: &str, value: Option<bool>) {
    if let Some(value) = value {
        lines.push(format!("{name}: {}", pass_label(value)));
    }
}

fn push_checked_gate_line(
    lines: &mut Vec<String>,
    name: &str,
    checked: Option<bool>,
    passed: Option<bool>,
) {
    match (checked, passed) {
        (Some(false), _) => lines.push(format!("{name}: not checked")),
        (_, Some(passed)) => lines.push(format!("{name}: {}", pass_label(passed))),
        (Some(true), None) => lines.push(format!("{name}: checked")),
        _ => {}
    }
}

fn push_applied_line(
    lines: &mut Vec<String>,
    name: &str,
    applied: Option<bool>,
    count: Option<&str>,
) {
    if let Some(count) = count.filter(|count| !count.is_empty()) {
        let passed = count.parse::<u64>().map(|value| value > 0).unwrap_or(false);
        lines.push(format!("{name}: {} count={count}", pass_label(passed)));
    } else {
        push_gate_line(lines, name, applied);
    }
}

fn push_count_line(lines: &mut Vec<String>, name: &str, count: Option<&str>) {
    let Some(count) = count.filter(|count| !count.is_empty()) else {
        return;
    };
    let passed = count.parse::<u64>().map(|value| value > 0).unwrap_or(false);
    lines.push(format!("{name}: {} count={count}", pass_label(passed)));
}

fn push_rust_check_line(lines: &mut Vec<String>, checked: Option<bool>, passed: Option<bool>) {
    match (checked, passed) {
        (Some(false), _) => lines.push("rust check: not checked".to_owned()),
        (_, Some(passed)) => lines.push(format!("rust check: {}", pass_label(passed))),
        (Some(true), None) => lines.push("rust check: checked".to_owned()),
        _ => {}
    }
}

fn push_runtime_value(parts: &mut Vec<String>, name: &str, value: Option<&str>) {
    if let Some(value) = value.filter(|value| !value.is_empty() && *value != "null") {
        parts.push(format!("{name}={}", short_preview(value, 80)));
    }
}

fn pass_label(value: bool) -> &'static str {
    if value { "PASS" } else { "FAIL" }
}

fn short_preview(value: &str, max_chars: usize) -> String {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut preview = normalized.chars().take(max_chars).collect::<String>();
    if normalized.chars().count() > max_chars {
        preview.push_str("...");
    }
    preview
}
