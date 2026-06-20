use super::{NotesCell, SessionCell, StoreCell};

pub(in crate::app::runtime_provider) fn settings(
    session: &SessionCell,
    store: &StoreCell,
    notes: &NotesCell,
) -> String {
    let mut summary = match session.lock() {
        Ok(session) => session.settings_summary(),
        Err(error) => format!("session settings unavailable: {error}"),
    };
    match store.lock() {
        Ok(store) => {
            if let Some(store) = store.as_ref() {
                summary.push_str(&format!(
                    " transcript={}",
                    store.transcript_path().display()
                ));
            } else {
                summary.push_str(" transcript=disabled");
            }
        }
        Err(error) => summary.push_str(&format!(" transcript=unavailable({error})")),
    }
    match notes.lock() {
        Ok(notes) => {
            if let Some(notes) = notes.as_ref() {
                summary.push_str(&format!(" {}", notes.summary()));
            } else {
                summary.push_str(" project_notes=disabled");
            }
        }
        Err(error) => summary.push_str(&format!(" project_notes=unavailable({error})")),
    }
    summary
}
