pub(super) fn business_answer_failure(
    has_runtime_model_experiences: bool,
    protocol_leak: bool,
    substituted_runtime_model_experiences: bool,
    evasive_denial: bool,
    handling_signal: bool,
    missing_signal: Option<&str>,
) -> Option<String> {
    if !has_runtime_model_experiences {
        return Some("answer did not include runtime_model_experiences evidence field".to_owned());
    }
    if protocol_leak {
        return Some("answer leaked hidden thought or channel protocol text".to_owned());
    }
    if substituted_runtime_model_experiences {
        return Some(
            "answer substituted runtime_model_experiences with memory_experiences".to_owned(),
        );
    }
    if evasive_denial {
        return Some("answer denied or avoided the local business runtime confirmation".to_owned());
    }
    if !handling_signal {
        return Some("answer did not include a business handling signal".to_owned());
    }
    missing_signal.map(|signal| format!("answer missing required business signal '{signal}'"))
}
