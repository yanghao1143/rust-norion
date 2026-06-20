use super::RustCheckFeedback;
use crate::gemma_business::response_json::{
    response_bool_field, response_ok, response_string_field, response_u64_field,
};

pub(super) fn rust_check_feedback_from_body(body: &str) -> RustCheckFeedback {
    let ok = response_ok(body)
        && response_bool_field(body, "passed")
        && response_string_field(body, "action") == "reinforce";
    RustCheckFeedback {
        checked: true,
        ok: Some(ok),
        applied: response_u64_field(body, "applied"),
    }
}
