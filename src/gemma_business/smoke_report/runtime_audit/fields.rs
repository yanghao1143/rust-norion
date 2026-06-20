use crate::gemma_business::response_json::response_u64_field;

pub(super) fn runtime_error_experiences(body: &str) -> u64 {
    response_u64_field(body, "runtime_error_experiences")
}

pub(super) fn runtime_errors(body: &str) -> u64 {
    response_u64_field(body, "runtime_errors")
}

pub(super) fn runtime_timeout_experiences(body: &str) -> u64 {
    response_u64_field(body, "runtime_timeout_experiences")
}

pub(super) fn runtime_timeouts(body: &str) -> u64 {
    response_u64_field(body, "runtime_timeouts")
}

pub(super) fn trace_runtime_error_events(body: &str) -> u64 {
    response_u64_field(body, "runtime_error_events")
}

pub(super) fn trace_runtime_timeout_events(body: &str) -> u64 {
    response_u64_field(body, "runtime_timeout_events")
}

#[cfg(test)]
mod tests {
    use super::{
        runtime_error_experiences, runtime_errors, runtime_timeout_experiences, runtime_timeouts,
        trace_runtime_error_events, trace_runtime_timeout_events,
    };

    #[test]
    fn runtime_audit_fields_keep_state_and_trace_names_distinct() {
        let body = "{\"runtime_error_experiences\":1,\"runtime_errors\":2,\"runtime_timeout_experiences\":3,\"runtime_timeouts\":4,\"runtime_error_events\":5,\"runtime_timeout_events\":6}";

        assert_eq!(runtime_error_experiences(body), 1);
        assert_eq!(runtime_errors(body), 2);
        assert_eq!(runtime_timeout_experiences(body), 3);
        assert_eq!(runtime_timeouts(body), 4);
        assert_eq!(trace_runtime_error_events(body), 5);
        assert_eq!(trace_runtime_timeout_events(body), 6);
    }
}
