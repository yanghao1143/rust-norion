mod context;
mod history;
mod index_notes;
mod notes;
mod options;
mod rust_check;
mod settings;

use std::sync::{Arc, Mutex};

use smartsteam_forge::{ForgeSession, ProjectNotesStore, SessionStore};

pub(super) use context::context_preview;
pub(super) use history::{new_session, record_event, resume_session, sessions, summarize_session};
pub(super) use index_notes::{
    append_index_context_query_status, retrieval_prompt_with_index_context,
};
pub(super) use notes::{
    add_model_pool_index_note, add_project_note, clear_model_pool_index_notes, clear_project_notes,
    model_pool_index_notes, project_notes, project_notes_context, set_project_notes,
};
pub(super) use options::{
    set_context_window, set_endpoint, set_feedback_amount, set_max_tokens, set_output, set_profile,
    set_self_improve,
};
pub(super) use rust_check::{
    clear_rust_check, set_rust_check_case, set_rust_check_edition, set_rust_check_file,
    set_rust_check_inline,
};
pub(super) use settings::settings;

type SessionCell = Arc<Mutex<ForgeSession>>;
type StoreCell = Arc<Mutex<Option<SessionStore>>>;
type NotesCell = Arc<Mutex<Option<ProjectNotesStore>>>;

pub(super) fn reset(session: &SessionCell) {
    if let Ok(mut session) = session.lock() {
        session.clear();
    }
}
