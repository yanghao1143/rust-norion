use super::{NotesCell, SessionCell};

pub(in crate::app::runtime_provider) fn context_preview(
    session: &SessionCell,
    notes: &NotesCell,
) -> Result<String, String> {
    let notes_context = match super::notes::project_notes_context(notes) {
        Ok(context) => context,
        Err(error) => return Err(format!("project notes unavailable: {error}")),
    };
    let mut session = session
        .lock()
        .map_err(|error| format!("session lock poisoned: {error}"))?;
    match notes_context {
        Some(context) => session.set_project_notes_context(context),
        None => session.clear_project_notes_context(),
    }
    Ok(session.context_preview())
}
