use super::GemmaModelServiceRuntimeAudit;

pub(super) fn runtime_audit_counters(
    audit: &GemmaModelServiceRuntimeAudit,
) -> [(&'static str, u64); 6] {
    [
        (
            "inspect state recorded runtime_error_experiences",
            audit.runtime_error_experiences,
        ),
        (
            "inspect state recorded runtime_errors",
            audit.runtime_errors,
        ),
        (
            "inspect state recorded runtime_timeout_experiences",
            audit.runtime_timeout_experiences,
        ),
        (
            "inspect state recorded runtime_timeouts",
            audit.runtime_timeouts,
        ),
        (
            "inspect trace recorded runtime_error_events",
            audit.trace_runtime_error_events,
        ),
        (
            "inspect trace recorded runtime_timeout_events",
            audit.trace_runtime_timeout_events,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::{GemmaModelServiceRuntimeAudit, runtime_audit_counters};

    #[test]
    fn runtime_audit_counters_preserve_failure_labels_and_values() {
        let audit = GemmaModelServiceRuntimeAudit {
            runtime_error_experiences: 1,
            runtime_errors: 2,
            runtime_timeout_experiences: 3,
            runtime_timeouts: 4,
            trace_runtime_error_events: 5,
            trace_runtime_timeout_events: 6,
        };

        assert_eq!(
            runtime_audit_counters(&audit),
            [
                ("inspect state recorded runtime_error_experiences", 1),
                ("inspect state recorded runtime_errors", 2),
                ("inspect state recorded runtime_timeout_experiences", 3),
                ("inspect state recorded runtime_timeouts", 4),
                ("inspect trace recorded runtime_error_events", 5),
                ("inspect trace recorded runtime_timeout_events", 6),
            ]
        );
    }
}
