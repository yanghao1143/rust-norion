mod counters;
mod failures;
mod fields;

use counters::runtime_audit_counters;
use failures::push_runtime_audit_failures;
use fields::{
    runtime_error_experiences, runtime_errors, runtime_timeout_experiences, runtime_timeouts,
    trace_runtime_error_events, trace_runtime_timeout_events,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct GemmaModelServiceRuntimeAudit {
    pub(crate) runtime_error_experiences: u64,
    pub(crate) runtime_errors: u64,
    pub(crate) runtime_timeout_experiences: u64,
    pub(crate) runtime_timeouts: u64,
    pub(crate) trace_runtime_error_events: u64,
    pub(crate) trace_runtime_timeout_events: u64,
}

impl GemmaModelServiceRuntimeAudit {
    pub(crate) fn from_inspect_body(body: &str) -> Self {
        Self {
            runtime_error_experiences: runtime_error_experiences(body),
            runtime_errors: runtime_errors(body),
            runtime_timeout_experiences: runtime_timeout_experiences(body),
            runtime_timeouts: runtime_timeouts(body),
            trace_runtime_error_events: trace_runtime_error_events(body),
            trace_runtime_timeout_events: trace_runtime_timeout_events(body),
        }
    }

    pub(crate) fn passed(&self) -> bool {
        runtime_audit_counters(self)
            .iter()
            .all(|(_, value)| *value == 0)
    }

    pub(crate) fn push_failures(&self, failures: &mut Vec<String>) {
        push_runtime_audit_failures(self, failures);
    }
}

#[cfg(test)]
mod tests {
    use super::GemmaModelServiceRuntimeAudit;

    #[test]
    fn runtime_audit_counters_drive_pass_and_failure_labels() {
        let audit = GemmaModelServiceRuntimeAudit {
            runtime_errors: 2,
            trace_runtime_timeout_events: 1,
            ..GemmaModelServiceRuntimeAudit::default()
        };
        let mut failures = Vec::new();

        assert!(!audit.passed());
        audit.push_failures(&mut failures);

        assert_eq!(failures.len(), 2);
        assert!(
            failures
                .iter()
                .any(|failure| failure == "inspect state recorded runtime_errors=2")
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure == "inspect trace recorded runtime_timeout_events=1")
        );
    }
}
