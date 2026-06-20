use std::sync::{Arc, Mutex, mpsc};

use smartsteam_forge::{FinalPayloadSummary, SessionAnswer, SessionStore};

use super::super::provider::ProviderEvent;

pub(super) fn record_answer(
    store: &Arc<Mutex<Option<SessionStore>>>,
    prompt: &str,
    answer: &SessionAnswer,
    tx: &mpsc::Sender<ProviderEvent>,
) {
    let result = with_store(store, |store| {
        store.append_message("user", prompt)?;
        if !answer.assistant_message.trim().is_empty() {
            store.append_message("assistant", &answer.assistant_message)?;
        }
        if let Some(payload) = &answer.final_payload {
            store.append_event("final_payload", payload)?;
            if let Some(report) = FinalPayloadSummary::parse(payload).gate_report() {
                store.append_event("gate_report", &report)?;
            }
        }
        Ok(())
    });
    if let Err(error) = result {
        let _ = tx.send(ProviderEvent::Status(format!("transcript error: {error}")));
    }
}

pub(super) fn record_error(
    store: &Arc<Mutex<Option<SessionStore>>>,
    prompt: &str,
    error: &str,
    tx: &mpsc::Sender<ProviderEvent>,
) {
    let result = with_store(store, |store| {
        store.append_message("user", prompt)?;
        store.append_event("error", error)
    });
    if let Err(error) = result {
        let _ = tx.send(ProviderEvent::Status(format!("transcript error: {error}")));
    }
}

pub(crate) fn with_store(
    store: &Arc<Mutex<Option<SessionStore>>>,
    action: impl FnOnce(&SessionStore) -> Result<(), String>,
) -> Result<(), String> {
    let store = store
        .lock()
        .map_err(|error| format!("session store lock poisoned: {error}"))?;
    let Some(store) = store.as_ref() else {
        return Ok(());
    };
    action(store)
}
