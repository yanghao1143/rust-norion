pub(super) fn is_gemma_smoke_generated_run_name(name: &str, base_name: &str) -> bool {
    let Some(suffix) = name
        .strip_prefix(base_name)
        .and_then(|rest| rest.strip_prefix('-'))
    else {
        return false;
    };
    !suffix.is_empty()
        && (suffix.bytes().all(|byte| byte.is_ascii_digit())
            || is_gemma_smoke_datetime_run_id(suffix))
}

fn is_gemma_smoke_datetime_run_id(suffix: &str) -> bool {
    let bytes = suffix.as_bytes();
    bytes.len() == 15
        && bytes[8] == b'-'
        && bytes[..8].iter().all(|byte| byte.is_ascii_digit())
        && bytes[9..].iter().all(|byte| byte.is_ascii_digit())
}
