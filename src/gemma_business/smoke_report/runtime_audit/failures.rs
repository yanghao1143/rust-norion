use super::GemmaModelServiceRuntimeAudit;
use super::counters::runtime_audit_counters;

pub(super) fn push_runtime_audit_failures(
    audit: &GemmaModelServiceRuntimeAudit,
    failures: &mut Vec<String>,
) {
    for (label, value) in runtime_audit_counters(audit) {
        push_runtime_audit_counter_failure(label, value, failures);
    }
}

fn push_runtime_audit_counter_failure(label: &str, value: u64, failures: &mut Vec<String>) {
    if value > 0 {
        failures.push(format!("{label}={value}"));
    }
}

#[cfg(test)]
mod tests {
    use super::push_runtime_audit_counter_failure;

    #[test]
    fn runtime_audit_counter_failure_only_records_nonzero_values() {
        let mut failures = Vec::new();

        push_runtime_audit_counter_failure("runtime_errors", 0, &mut failures);
        push_runtime_audit_counter_failure("runtime_timeouts", 3, &mut failures);

        assert_eq!(failures, ["runtime_timeouts=3"]);
    }
}
