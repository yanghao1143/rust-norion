use smartsteam_forge::StreamEndpoint;

use super::SessionCell;

pub(in crate::app::runtime_provider) fn set_endpoint(
    session: &SessionCell,
    endpoint: StreamEndpoint,
) -> Result<(), String> {
    let mut session = session
        .lock()
        .map_err(|error| format!("session lock poisoned: {error}"))?;
    session.set_endpoint(endpoint);
    Ok(())
}

pub(in crate::app::runtime_provider) fn set_output(
    session: &SessionCell,
    output: &str,
) -> Result<(), String> {
    let mut session = session
        .lock()
        .map_err(|error| format!("session lock poisoned: {error}"))?;
    session.set_output(output);
    Ok(())
}

pub(in crate::app::runtime_provider) fn set_profile(
    session: &SessionCell,
    profile: &str,
) -> Result<(), String> {
    let mut session = session
        .lock()
        .map_err(|error| format!("session lock poisoned: {error}"))?;
    session.set_profile(profile);
    Ok(())
}

pub(in crate::app::runtime_provider) fn set_feedback_amount(
    session: &SessionCell,
    amount: &str,
) -> Result<(), String> {
    let mut session = session
        .lock()
        .map_err(|error| format!("session lock poisoned: {error}"))?;
    session.set_feedback_amount(amount);
    Ok(())
}

pub(in crate::app::runtime_provider) fn set_self_improve(
    session: &SessionCell,
    enabled: bool,
) -> Result<(), String> {
    let mut session = session
        .lock()
        .map_err(|error| format!("session lock poisoned: {error}"))?;
    session.set_self_improve(enabled);
    Ok(())
}

pub(in crate::app::runtime_provider) fn set_context_window(
    session: &SessionCell,
    max_messages: usize,
) -> Result<String, String> {
    let mut session = session
        .lock()
        .map_err(|error| format!("session lock poisoned: {error}"))?;
    session.set_max_context_messages(max_messages);
    Ok(session.context_budget_summary())
}

pub(in crate::app::runtime_provider) fn set_max_tokens(
    session: &SessionCell,
    max_tokens: Option<usize>,
) -> Result<String, String> {
    let mut session = session
        .lock()
        .map_err(|error| format!("session lock poisoned: {error}"))?;
    session.set_max_tokens(max_tokens);
    Ok(format!(
        "max_tokens={}",
        max_tokens
            .map(|value| value.to_string())
            .unwrap_or_else(|| "backend-default".to_owned())
    ))
}
