use smartsteam_forge::{SessionFilter, SessionStore};

use super::super::transcript::with_store;
use super::{SessionCell, StoreCell, reset};

pub(in crate::app::runtime_provider) fn sessions(
    store: &StoreCell,
    filter: SessionFilter,
    limit: usize,
) -> String {
    let store = match store.lock() {
        Ok(store) => store,
        Err(error) => return format!("session listing unavailable: {error}"),
    };
    let Some(store) = store.as_ref() else {
        return "session store disabled".to_owned();
    };
    let limit = limit.clamp(1, 500);
    match store.list_recent_filtered(filter, limit) {
        Ok(records) if records.is_empty() => {
            format!("no recorded sessions for filter={}", filter.label())
        }
        Ok(records) => {
            let lines = records
                .iter()
                .enumerate()
                .map(|(index, record)| format!("{}. {}", index + 1, record.summary_line()))
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "recent sessions filter={} limit={}:\n{lines}",
                filter.label(),
                limit
            )
        }
        Err(error) => format!("list sessions failed: {error}"),
    }
}

pub(in crate::app::runtime_provider) fn resume_session(
    session: &SessionCell,
    store: &StoreCell,
    selector: &str,
) -> Result<String, String> {
    let mut store = store
        .lock()
        .map_err(|error| format!("session store lock poisoned: {error}"))?;
    if store.is_none() {
        *store = Some(SessionStore::open_default()?);
    }
    let Some(store) = store.as_mut() else {
        return Err("session store unavailable".to_owned());
    };
    let max_messages = match session.lock() {
        Ok(session) => session.settings().max_context_messages,
        Err(error) => return Err(format!("session lock poisoned: {error}")),
    };
    let resumed = store.resume(selector, max_messages)?;
    let summary = store.summarize_current()?;
    let summary_context = summary.to_context_prompt();
    let loaded_messages = resumed.messages.len();
    let summary_context_chars;
    {
        let mut session = session
            .lock()
            .map_err(|error| format!("session lock poisoned: {error}"))?;
        session.load_transcript_messages(resumed.messages);
        session.set_summary_context(summary_context);
        summary_context_chars = session.summary_context_chars();
    }
    Ok(format!(
        "session={} loaded_messages={} summary_context_chars={} transcript={} summary={}",
        resumed.record.id,
        loaded_messages,
        summary_context_chars,
        resumed.record.transcript_path.display(),
        summary.summary_path.display()
    ))
}

pub(in crate::app::runtime_provider) fn summarize_session(
    store: &StoreCell,
    selector: &str,
) -> Result<String, String> {
    let store = store
        .lock()
        .map_err(|error| format!("session store lock poisoned: {error}"))?;
    let Some(store) = store.as_ref() else {
        return Err("session store disabled".to_owned());
    };
    let summary = store.summarize(selector)?;
    Ok(summary.summary_line())
}

pub(in crate::app::runtime_provider) fn record_event(
    store: &StoreCell,
    kind: &str,
    content: &str,
) -> Result<(), String> {
    with_store(store, |store| store.append_event(kind, content))
}

pub(in crate::app::runtime_provider) fn new_session(
    session: &SessionCell,
    store: &StoreCell,
) -> Result<String, String> {
    reset(session);
    let mut store = store
        .lock()
        .map_err(|error| format!("session store lock poisoned: {error}"))?;
    if store.is_none() {
        *store = Some(SessionStore::open_default()?);
    }
    let Some(store) = store.as_mut() else {
        return Err("session store unavailable".to_owned());
    };
    let session = store.rotate()?;
    Ok(format!("transcript={}", session.transcript_path.display()))
}
