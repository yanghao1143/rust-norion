use smartsteam_forge::ProjectNotesStore;

use super::{NotesCell, SessionCell};

pub(in crate::app::runtime_provider) fn project_notes(notes: &NotesCell) -> Result<String, String> {
    let store = notes_store(notes)?;
    let content = store.read()?;
    if content.trim().is_empty() {
        Ok(format!("Project notes are empty.\n{}", store.summary()))
    } else {
        Ok(format!("{}\n\n{}", store.summary(), content.trim_end()))
    }
}

pub(in crate::app::runtime_provider) fn add_project_note(
    session: &SessionCell,
    notes: &NotesCell,
    note: &str,
) -> Result<String, String> {
    let store = notes_store(notes)?;
    let summary = store.append(note)?;
    sync_session_project_notes(session, &store)?;
    Ok(summary)
}

pub(in crate::app::runtime_provider) fn add_model_pool_index_note(
    session: &SessionCell,
    notes: &NotesCell,
    note: &str,
) -> Result<String, String> {
    let store = notes_store(notes)?;
    let summary = store.append_model_pool_index_note(note)?;
    sync_session_project_notes(session, &store)?;
    Ok(summary)
}

pub(in crate::app::runtime_provider) fn set_project_notes(
    session: &SessionCell,
    notes: &NotesCell,
    content: &str,
) -> Result<String, String> {
    let store = notes_store(notes)?;
    let summary = store.write(content)?;
    sync_session_project_notes(session, &store)?;
    Ok(summary)
}

pub(in crate::app::runtime_provider) fn clear_project_notes(
    session: &SessionCell,
    notes: &NotesCell,
) -> Result<String, String> {
    let store = notes_store(notes)?;
    let summary = store.clear()?;
    if let Ok(mut session) = session.lock() {
        session.clear_project_notes_context();
    }
    Ok(summary)
}

pub(in crate::app::runtime_provider) fn model_pool_index_notes(
    notes: &NotesCell,
) -> Result<String, String> {
    notes_store(notes)?.model_pool_index_notes()
}

pub(in crate::app::runtime_provider) fn clear_model_pool_index_notes(
    session: &SessionCell,
    notes: &NotesCell,
) -> Result<String, String> {
    let store = notes_store(notes)?;
    let summary = store.clear_model_pool_index_notes()?;
    sync_session_project_notes(session, &store)?;
    Ok(summary)
}

pub(in crate::app::runtime_provider) fn project_notes_context(
    notes: &NotesCell,
) -> Result<Option<String>, String> {
    notes_store(notes)?.read_context()
}

fn sync_session_project_notes(
    session: &SessionCell,
    store: &ProjectNotesStore,
) -> Result<(), String> {
    let context = store.read_context()?;
    if let Ok(mut session) = session.lock() {
        match context {
            Some(context) => session.set_project_notes_context(context),
            None => session.clear_project_notes_context(),
        }
    }
    Ok(())
}

fn notes_store(notes: &NotesCell) -> Result<ProjectNotesStore, String> {
    notes
        .lock()
        .map_err(|error| format!("project notes lock poisoned: {error}"))?
        .clone()
        .ok_or_else(|| "project notes store unavailable".to_owned())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use smartsteam_forge::{ForgeSession, session::ModelPoolIndexNoteActive};

    use super::*;

    #[test]
    fn clear_model_pool_index_notes_syncs_session_context() {
        let root = std::env::temp_dir().join(format!(
            "smartsteam_runtime_index_clear_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let store = ProjectNotesStore::open(root.join("project_notes.md"));
        let session = Arc::new(Mutex::new(ForgeSession::default()));
        let notes = Arc::new(Mutex::new(Some(store.clone())));

        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            store.path(),
            concat!(
                "manual note\n",
                "model_pool_index:\n",
                "source_prompt: repo map\n",
                "selected_role: index\n",
                "selected_base_url: http://127.0.0.1:8687\n",
                "answer:\n",
                "LATEST_INDEX src/model_service\n",
                "model_pool_index_end:\n",
                "model_pool_index:\n",
                "LEGACY_STALE src/old\n"
            ),
        )
        .unwrap();
        sync_session_project_notes(&session, &store).unwrap();
        assert_eq!(
            session.lock().unwrap().model_pool_index_note_stats().active,
            ModelPoolIndexNoteActive::LatestDelimited
        );

        let summary = clear_model_pool_index_notes(&session, &notes).unwrap();

        assert!(summary.contains("removed=2"));
        assert!(summary.contains("legacy_undelimited=1"));
        assert_eq!(
            session.lock().unwrap().model_pool_index_note_stats().active,
            ModelPoolIndexNoteActive::None
        );
        assert!(!store.read().unwrap().contains("LEGACY_STALE"));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn manual_notes_do_not_create_model_pool_index_context() {
        let root = std::env::temp_dir().join(format!(
            "smartsteam_runtime_manual_index_escape_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let store = ProjectNotesStore::open(root.join("project_notes.md"));
        let session = Arc::new(Mutex::new(ForgeSession::default()));
        let notes = Arc::new(Mutex::new(Some(store.clone())));

        set_project_notes(
            &session,
            &notes,
            concat!(
                "manual set\n",
                "model_pool_index:\n",
                "answer:\n",
                "FAKE_SET_INDEX src/old\n",
                "model_pool_index_end:\n"
            ),
        )
        .unwrap();
        add_project_note(
            &session,
            &notes,
            concat!(
                "manual append\n",
                "model_pool_index:\n",
                "answer:\n",
                "FAKE_APPEND_INDEX src/new\n",
                "model_pool_index_end:\n"
            ),
        )
        .unwrap();

        let content = store.read().unwrap();
        assert!(content.contains("FAKE_SET_INDEX src/old"));
        assert!(content.contains("FAKE_APPEND_INDEX src/new"));
        assert!(content.contains("[escaped model_pool_index marker] model_pool_index:"));
        assert_eq!(
            session.lock().unwrap().model_pool_index_note_stats().active,
            ModelPoolIndexNoteActive::None
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn trusted_model_pool_index_note_creates_index_context() {
        let root = std::env::temp_dir().join(format!(
            "smartsteam_runtime_trusted_index_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let store = ProjectNotesStore::open(root.join("project_notes.md"));
        let session = Arc::new(Mutex::new(ForgeSession::default()));
        let notes = Arc::new(Mutex::new(Some(store.clone())));

        add_model_pool_index_note(
            &session,
            &notes,
            concat!(
                "model_pool_index:\n",
                "source_prompt: repo map\n",
                "selected_role: index\n",
                "selected_base_url: http://127.0.0.1:8687\n",
                "answer:\n",
                "TRUSTED_INDEX src/model_service\n",
                "model_pool_index_end:\n"
            ),
        )
        .unwrap();

        assert_eq!(
            session.lock().unwrap().model_pool_index_note_stats().active,
            ModelPoolIndexNoteActive::LatestDelimited
        );
        assert!(
            session
                .lock()
                .unwrap()
                .context_preview()
                .contains("model_pool_index_context: blocks=1 delimited=1")
        );

        let _ = std::fs::remove_dir_all(root);
    }
}
