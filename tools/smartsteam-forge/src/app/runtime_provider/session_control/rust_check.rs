use std::{fs, path::Path};

use super::SessionCell;

pub(in crate::app::runtime_provider) fn set_rust_check_inline(
    session: &SessionCell,
    code: &str,
) -> Result<String, String> {
    let mut session = session
        .lock()
        .map_err(|error| format!("session lock poisoned: {error}"))?;
    session.set_rust_check_code(code);
    Ok(format!(
        "rust_check inline loaded chars={}",
        code.chars().count()
    ))
}

pub(in crate::app::runtime_provider) fn set_rust_check_file(
    session: &SessionCell,
    path: &str,
) -> Result<String, String> {
    let code = fs::read_to_string(Path::new(path))
        .map_err(|error| format!("read rust-check file {path} failed: {error}"))?;
    let mut session = session
        .lock()
        .map_err(|error| format!("session lock poisoned: {error}"))?;
    session.set_rust_check_code(&code);
    Ok(format!(
        "rust_check file={} loaded chars={}",
        path,
        code.chars().count()
    ))
}

pub(in crate::app::runtime_provider) fn set_rust_check_edition(
    session: &SessionCell,
    edition: &str,
) -> Result<(), String> {
    let mut session = session
        .lock()
        .map_err(|error| format!("session lock poisoned: {error}"))?;
    session.set_rust_check_edition(edition);
    Ok(())
}

pub(in crate::app::runtime_provider) fn set_rust_check_case(
    session: &SessionCell,
    case_name: Option<String>,
) -> Result<(), String> {
    let mut session = session
        .lock()
        .map_err(|error| format!("session lock poisoned: {error}"))?;
    session.set_rust_check_case(case_name);
    Ok(())
}

pub(in crate::app::runtime_provider) fn clear_rust_check(
    session: &SessionCell,
) -> Result<(), String> {
    let mut session = session
        .lock()
        .map_err(|error| format!("session lock poisoned: {error}"))?;
    session.clear_rust_check_code();
    Ok(())
}
