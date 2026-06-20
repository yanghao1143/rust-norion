mod hints;

#[cfg(test)]
mod tests;

use std::io::{self, Write};

use super::provider::ChatProvider;
use hints::diagnostic_hints;

pub fn run_diagnostic(provider: &dyn ChatProvider) -> io::Result<()> {
    let report = build_diagnostic_report(provider);
    let _ = provider.record_event("diagnostic_report", &report);
    let mut stdout = io::stdout();
    writeln!(stdout, "{report}")?;
    stdout.flush()
}

pub fn build_diagnostic_report(provider: &dyn ChatProvider) -> String {
    let (health, readiness, safe_device) = provider.health_readiness_and_safe_device();
    let mut lines = vec![
        "SmartSteam Forge doctor".to_owned(),
        format!("target/backend: {}", provider.diagnostic_target()),
        check_line("health", &health),
        check_line("readiness", &readiness),
        check_line("safe-device", &safe_device),
        "next steps:".to_owned(),
    ];

    lines.extend(
        diagnostic_hints(&health, &readiness, &safe_device)
            .into_iter()
            .map(|hint| format!("- {hint}")),
    );
    lines.join("\n")
}

fn check_line(name: &str, result: &Result<String, String>) -> String {
    match result {
        Ok(summary) => format!("{name}: PASS {summary}"),
        Err(error) => format!("{name}: FAIL {error}"),
    }
}
