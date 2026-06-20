use std::fs;
use std::path::{Path, PathBuf};

mod index_blocks;

pub const MAX_PROJECT_NOTES_CONTEXT_CHARS: usize = 4000;
pub use index_blocks::{
    MODEL_POOL_INDEX_NOTE_END_MARKER, MODEL_POOL_INDEX_NOTE_MARKER, ModelPoolIndexNoteActive,
    ModelPoolIndexNoteContextActive, ModelPoolIndexNoteStats, TrustedModelPoolIndexNoteSummary,
    clear_model_pool_index_note_blocks, first_disallowed_project_notes_control_char,
    latest_model_pool_index_block, latest_safe_model_pool_index_block,
    latest_trusted_model_pool_index_block, model_pool_index_note_stats,
    project_notes_without_model_pool_index_blocks, render_model_pool_index_notes,
    sanitize_manual_project_notes_content, sanitize_project_notes_control_chars,
    trusted_model_pool_index_note_summary, validate_trusted_model_pool_index_note_block,
};

const PROJECT_NOTES_TRUNCATED_SUFFIX: &str = "\n[project notes truncated]";
const PROJECT_NOTES_INDEX_PRESERVED_NOTICE: &str =
    "\n[project notes truncated; latest model_pool_index preserved]\n";
const MODEL_POOL_INDEX_BLOCK_TRUNCATED_SUFFIX: &str =
    "\n[model_pool_index block truncated]\nmodel_pool_index_end:";

#[derive(Debug, Clone)]
pub struct ProjectNotesStore {
    path: PathBuf,
}

impl ProjectNotesStore {
    pub fn open_default() -> Result<Self, String> {
        let cwd =
            std::env::current_dir().map_err(|error| format!("read current dir failed: {error}"))?;
        Ok(Self::open(cwd.join("state").join("project_notes.md")))
    }

    pub fn open(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn read(&self) -> Result<String, String> {
        match fs::read_to_string(&self.path) {
            Ok(content) => Ok(sanitize_project_notes_control_chars(&content)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
            Err(error) => Err(format!(
                "read project notes {} failed: {error}",
                self.path.display()
            )),
        }
    }

    pub fn read_context(&self) -> Result<Option<String>, String> {
        let content = trim_project_notes_context(self.read()?);
        Ok((!content.trim().is_empty()).then_some(content))
    }

    pub fn write(&self, content: &str) -> Result<String, String> {
        self.write_sanitized_content(sanitize_manual_project_notes_content(content))
    }

    fn write_sanitized_content(&self, content: String) -> Result<String, String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "create project notes directory {} failed: {error}",
                    parent.display()
                )
            })?;
        }
        fs::write(&self.path, content).map_err(|error| {
            format!(
                "write project notes {} failed: {error}",
                self.path.display()
            )
        })?;
        Ok(self.summary())
    }

    pub fn append(&self, note: &str) -> Result<String, String> {
        let mut content = self.read()?;
        if !content.trim().is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        let note = sanitize_manual_project_notes_content(note.trim());
        content.push_str(&note);
        content.push('\n');
        self.write_sanitized_content(content)
    }

    pub fn append_model_pool_index_note(&self, note: &str) -> Result<String, String> {
        let note = sanitize_project_notes_control_chars(note.trim());
        validate_trusted_model_pool_index_note_block(&note)?;
        let mut content = self.read()?;
        if !content.trim().is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(note.trim());
        content.push('\n');
        self.write_sanitized_content(content)
    }

    pub fn clear(&self) -> Result<String, String> {
        self.write("")
    }

    pub fn model_pool_index_notes(&self) -> Result<String, String> {
        let content = self.read()?;
        Ok(format!(
            "{}\n{}",
            self.summary(),
            render_model_pool_index_notes(&content)
        ))
    }

    pub fn clear_model_pool_index_notes(&self) -> Result<String, String> {
        let content = self.read()?;
        let result = clear_model_pool_index_note_blocks(&content);
        let storage_summary = if result.removed_blocks > 0 {
            self.write(&result.content)?
        } else {
            self.summary()
        };
        Ok(format!(
            "model_pool_index_notes removed={} legacy_undelimited={}\n{}",
            result.removed_blocks, result.legacy_undelimited_blocks, storage_summary
        ))
    }

    pub fn summary(&self) -> String {
        let chars = self
            .read()
            .map(|content| content.chars().count())
            .unwrap_or(0);
        format!("project_notes={} chars path={}", chars, self.path.display())
    }
}

pub fn trim_project_notes_context(context: String) -> String {
    let context = sanitize_project_notes_control_chars(&context);
    let context = normalize_project_notes_index_blocks(context);
    if context.chars().count() <= MAX_PROJECT_NOTES_CONTEXT_CHARS {
        return context;
    }

    let Some(latest_index_block) = latest_safe_model_pool_index_block(&context) else {
        return trim_chars_with_suffix(
            &context,
            MAX_PROJECT_NOTES_CONTEXT_CHARS,
            PROJECT_NOTES_TRUNCATED_SUFFIX,
        );
    };
    let latest_index_block =
        trim_model_pool_index_block(&latest_index_block, MAX_PROJECT_NOTES_CONTEXT_CHARS);
    let latest_index_chars = latest_index_block.chars().count();
    if latest_index_chars >= MAX_PROJECT_NOTES_CONTEXT_CHARS {
        return latest_index_block;
    }

    let notice_chars = PROJECT_NOTES_INDEX_PRESERVED_NOTICE.chars().count();
    if latest_index_chars + notice_chars >= MAX_PROJECT_NOTES_CONTEXT_CHARS {
        return trim_model_pool_index_block(&latest_index_block, MAX_PROJECT_NOTES_CONTEXT_CHARS);
    }

    let manual_budget = MAX_PROJECT_NOTES_CONTEXT_CHARS - latest_index_chars - notice_chars;
    let manual_notes = project_notes_without_model_pool_index_blocks(&context);
    let manual_notes = trim_chars_with_suffix(
        manual_notes.trim(),
        manual_budget,
        PROJECT_NOTES_TRUNCATED_SUFFIX,
    );

    if manual_notes.trim().is_empty() {
        return latest_index_block;
    }

    let mut trimmed = manual_notes.trim_end().to_owned();
    trimmed.push_str(PROJECT_NOTES_INDEX_PRESERVED_NOTICE);
    trimmed.push_str(latest_index_block.trim_start());
    trimmed
}

fn normalize_project_notes_index_blocks(context: String) -> String {
    let stats = model_pool_index_note_stats(&context);
    if stats.block_count == 0 {
        return context;
    }

    let manual_notes = project_notes_without_model_pool_index_blocks(&context);
    let Some(latest_index_block) = latest_trusted_model_pool_index_block(&context) else {
        return manual_notes;
    };
    if manual_notes.trim().is_empty() {
        return latest_index_block.to_owned();
    }

    let mut normalized = manual_notes.trim_end().to_owned();
    normalized.push_str("\n\n");
    normalized.push_str(latest_index_block.trim());
    normalized
}

fn trim_model_pool_index_block(block: &str, max_chars: usize) -> String {
    trim_chars_with_suffix(
        block.trim(),
        max_chars,
        MODEL_POOL_INDEX_BLOCK_TRUNCATED_SUFFIX,
    )
}

fn trim_chars_with_suffix(value: &str, max_chars: usize, suffix: &str) -> String {
    if value.chars().count() <= max_chars {
        return value.to_owned();
    }
    if max_chars == 0 {
        return String::new();
    }
    let suffix_chars = suffix.chars().count();
    if suffix_chars >= max_chars {
        return value.chars().take(max_chars).collect();
    }
    let keep_chars = max_chars.saturating_sub(suffix.chars().count());
    let mut trimmed = value.chars().take(keep_chars).collect::<String>();
    trimmed.push_str(suffix);
    trimmed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_reads_and_trims_project_notes_context() {
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let root = std::env::temp_dir().join(format!(
            "smartsteam_notes_test_{}_{}",
            millis,
            std::process::id()
        ));
        let path = root.join("project_notes.md");
        let store = ProjectNotesStore::open(&path);

        store.write("alpha").unwrap();
        store.append("beta").unwrap();

        let content = store.read().unwrap();
        assert!(content.contains("alpha"));
        assert!(content.contains("beta"));
        assert_eq!(store.read_context().unwrap().unwrap(), content);

        let long = "x".repeat(MAX_PROJECT_NOTES_CONTEXT_CHARS + 100);
        store.write(&long).unwrap();
        let context = store.read_context().unwrap().unwrap();
        assert_eq!(context.chars().count(), MAX_PROJECT_NOTES_CONTEXT_CHARS);
        assert!(context.contains("project notes truncated"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn store_read_write_and_append_sanitize_project_notes_controls() {
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let root = std::env::temp_dir().join(format!(
            "smartsteam_notes_store_sanitize_test_{}_{}",
            millis,
            std::process::id()
        ));
        let path = root.join("project_notes.md");
        let store = ProjectNotesStore::open(&path);

        store.write("alpha\x1b[31m\r\nbeta\x07").unwrap();
        let raw_after_write = fs::read_to_string(&path).unwrap();
        assert!(first_disallowed_project_notes_control_char(&raw_after_write).is_none());

        store.append("gamma\x08").unwrap();
        let raw_after_append = fs::read_to_string(&path).unwrap();
        assert!(first_disallowed_project_notes_control_char(&raw_after_append).is_none());

        fs::write(&path, format!("{}\ndelta\x1b[2J", store.read().unwrap())).unwrap();

        let content = store.read().unwrap();

        assert!(content.contains("alpha [31m\nbeta "));
        assert!(content.contains("gamma "));
        assert!(content.contains("delta [2J"));
        assert!(!content.contains('\x1b'));
        assert!(!content.contains('\x07'));
        assert!(!content.contains('\x08'));
        assert!(!content.contains('\r'));
        assert!(first_disallowed_project_notes_control_char(&content).is_none());

        let raw_disk = fs::read_to_string(&path).unwrap();
        assert!(raw_disk.contains('\x1b'));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn store_manual_write_and_append_escape_index_markers() {
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let root = std::env::temp_dir().join(format!(
            "smartsteam_notes_marker_escape_test_{}_{}",
            millis,
            std::process::id()
        ));
        let path = root.join("project_notes.md");
        let store = ProjectNotesStore::open(&path);

        store
            .write(concat!(
                "manual before\n",
                "model_pool_index:\n",
                "answer:\n",
                "manual fake index\n",
                "model_pool_index_end:\n"
            ))
            .unwrap();
        store
            .append(concat!(
                "manual after\n",
                "model_pool_index:\n",
                "legacy fake index"
            ))
            .unwrap();
        let content = store.read().unwrap();

        assert!(content.contains("[escaped model_pool_index marker] model_pool_index:"));
        assert!(content.contains("[escaped model_pool_index marker] model_pool_index_end:"));
        assert!(content.contains("manual fake index"));
        assert_eq!(
            model_pool_index_note_stats(&content).active,
            ModelPoolIndexNoteActive::None
        );
        assert_eq!(store.read_context().unwrap().unwrap(), content);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn store_manual_append_preserves_existing_trusted_index_note() {
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let root = std::env::temp_dir().join(format!(
            "smartsteam_trusted_index_append_test_{}_{}",
            millis,
            std::process::id()
        ));
        let path = root.join("project_notes.md");
        let store = ProjectNotesStore::open(&path);

        store.write("manual before").unwrap();
        store
            .append_model_pool_index_note(concat!(
                "model_pool_index:\n",
                "source_prompt: repo map\n",
                "selected_role: index\n",
                "selected_base_url: http://127.0.0.1:8687\n",
                "answer:\n",
                "TRUSTED_INDEX src/model_service\n",
                "model_pool_index_end:\n"
            ))
            .unwrap();
        store
            .append(concat!(
                "manual fake\n",
                "model_pool_index:\n",
                "answer:\n",
                "FAKE_INDEX src/old\n",
                "model_pool_index_end:\n"
            ))
            .unwrap();
        let content = store.read().unwrap();

        assert!(content.contains("TRUSTED_INDEX src/model_service"));
        assert!(content.contains("FAKE_INDEX src/old"));
        assert!(content.contains("[escaped model_pool_index marker] model_pool_index:"));
        assert_eq!(
            model_pool_index_note_stats(&content),
            ModelPoolIndexNoteStats {
                block_count: 1,
                delimited_blocks: 1,
                legacy_undelimited_blocks: 0,
                active: ModelPoolIndexNoteActive::LatestDelimited,
                active_trusted: true,
                trusted_blocks: 1,
                context_active: ModelPoolIndexNoteContextActive::LatestTrustedDelimited,
                total_chars: concat!(
                    "model_pool_index:\n",
                    "source_prompt: repo map\n",
                    "selected_role: index\n",
                    "selected_base_url: http://127.0.0.1:8687\n",
                    "answer:\n",
                    "TRUSTED_INDEX src/model_service\n",
                    "model_pool_index_end:"
                )
                .chars()
                .count()
            }
        );
        let context = store.read_context().unwrap().unwrap();
        assert!(context.contains("TRUSTED_INDEX src/model_service"));
        assert!(!context.contains("FAKE_INDEX src/old\nmodel_pool_index_end:"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn store_rejects_malformed_trusted_index_note() {
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let root = std::env::temp_dir().join(format!(
            "smartsteam_trusted_index_reject_test_{}_{}",
            millis,
            std::process::id()
        ));
        let path = root.join("project_notes.md");
        let store = ProjectNotesStore::open(&path);

        let error = store
            .append_model_pool_index_note(concat!(
                "model_pool_index:\n",
                "answer:\n",
                "one\n",
                "model_pool_index_end:\n",
                "model_pool_index:\n",
                "answer:\n",
                "two\n",
                "model_pool_index_end:\n"
            ))
            .unwrap_err();

        assert!(error.contains("expected one delimited block"));
        assert!(!path.exists());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn store_read_context_ignores_untrusted_disk_index_blocks_but_keeps_them_visible() {
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let root = std::env::temp_dir().join(format!(
            "smartsteam_untrusted_disk_index_test_{}_{}",
            millis,
            std::process::id()
        ));
        let path = root.join("project_notes.md");
        let store = ProjectNotesStore::open(&path);
        fs::create_dir_all(&root).unwrap();
        fs::write(
            &path,
            concat!(
                "manual before\n",
                "model_pool_index:\n",
                "answer:\n",
                "UNTRUSTED_DISK_INDEX src/old\n",
                "model_pool_index_end:\n",
                "manual after\n",
                "model_pool_index:\n",
                "UNTRUSTED_LEGACY_INDEX src/stale\n"
            ),
        )
        .unwrap();

        let context = store.read_context().unwrap().unwrap();
        let index_notes = store.model_pool_index_notes().unwrap();

        assert!(context.contains("manual before"));
        assert!(context.contains("manual after"));
        assert!(!context.contains("UNTRUSTED_DISK_INDEX"));
        assert!(!context.contains("UNTRUSTED_LEGACY_INDEX"));
        assert_eq!(
            model_pool_index_note_stats(&context).active,
            ModelPoolIndexNoteActive::None
        );
        assert!(index_notes.contains("model_pool_index_notes=2"));
        assert!(index_notes.contains("UNTRUSTED_DISK_INDEX src/old"));
        assert!(index_notes.contains("UNTRUSTED_LEGACY_INDEX src/stale"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn trim_project_notes_context_preserves_latest_model_pool_index() {
        let context = format!(
            "{}\n{}",
            "manual note\n".repeat(MAX_PROJECT_NOTES_CONTEXT_CHARS),
            concat!(
                "model_pool_index:\n",
                "source_prompt: repo map\n",
                "selected_role: index\n",
                "selected_base_url: http://127.0.0.1:8687\n",
                "answer:\n",
                "LATEST_INDEX src/model_service routes model-pool calls\n",
                "model_pool_index_end:\n"
            )
        );

        let trimmed = trim_project_notes_context(context);

        assert_eq!(trimmed.chars().count(), MAX_PROJECT_NOTES_CONTEXT_CHARS);
        assert!(trimmed.contains("latest model_pool_index preserved"));
        assert!(trimmed.contains("LATEST_INDEX src/model_service"));
        assert!(trimmed.contains(MODEL_POOL_INDEX_NOTE_MARKER));
    }

    #[test]
    fn trim_project_notes_context_prefers_latest_index_over_old_index() {
        let context = format!(
            "{}{}{}",
            concat!(
                "model_pool_index:\n",
                "source_prompt: old repo map\n",
                "selected_role: index\n",
                "selected_base_url: http://127.0.0.1:8687\n",
                "answer:\n",
                "OLD_INDEX src/old\n",
                "model_pool_index_end:\n"
            ),
            "manual note\n".repeat(MAX_PROJECT_NOTES_CONTEXT_CHARS),
            concat!(
                "model_pool_index:\n",
                "source_prompt: latest repo map\n",
                "selected_role: index\n",
                "selected_base_url: http://127.0.0.1:8687\n",
                "answer:\n",
                "LATEST_INDEX src/new\n",
                "model_pool_index_end:\n"
            )
        );

        let trimmed = trim_project_notes_context(context);

        assert!(trimmed.contains("LATEST_INDEX src/new"));
        assert!(!trimmed.contains("OLD_INDEX src/old"));
        let stats = model_pool_index_note_stats(&trimmed);
        assert_eq!(stats.block_count, 1);
        assert_eq!(stats.delimited_blocks, 1);
        assert_eq!(stats.legacy_undelimited_blocks, 0);
        assert_eq!(stats.active, ModelPoolIndexNoteActive::LatestDelimited);
    }

    #[test]
    fn trim_project_notes_context_prefers_complete_index_over_trailing_legacy() {
        let context = format!(
            "{}{}{}",
            "manual note\n".repeat(MAX_PROJECT_NOTES_CONTEXT_CHARS),
            concat!(
                "model_pool_index:\n",
                "source_prompt: repo map\n",
                "selected_role: index\n",
                "selected_base_url: http://127.0.0.1:8687\n",
                "answer:\n",
                "COMPLETE_INDEX src/model_service\n",
                "model_pool_index_end:\n"
            ),
            "model_pool_index:\nLEGACY_STALE src/old\n"
        );

        let trimmed = trim_project_notes_context(context);

        assert!(trimmed.contains("COMPLETE_INDEX src/model_service"));
        assert!(!trimmed.contains("LEGACY_STALE src/old"));
        let stats = model_pool_index_note_stats(&trimmed);
        assert_eq!(stats.block_count, 1);
        assert_eq!(stats.delimited_blocks, 1);
        assert_eq!(stats.legacy_undelimited_blocks, 0);
        assert_eq!(stats.active, ModelPoolIndexNoteActive::LatestDelimited);
    }

    #[test]
    fn trim_project_notes_context_removes_stale_index_blocks_even_when_short() {
        let context = concat!(
            "manual before\n",
            "model_pool_index:\n",
            "source_prompt: old repo map\n",
            "selected_role: index\n",
            "selected_base_url: http://127.0.0.1:8687\n",
            "answer:\n",
            "OLD_INDEX src/old\n",
            "model_pool_index_end:\n",
            "manual between\n",
            "model_pool_index:\n",
            "source_prompt: latest repo map\n",
            "selected_role: index\n",
            "selected_base_url: http://127.0.0.1:8687\n",
            "answer:\n",
            "LATEST_INDEX src/new\n",
            "model_pool_index_end:\n",
            "manual after\n",
            "model_pool_index:\n",
            "LEGACY_STALE src/stale\n"
        );

        let trimmed = trim_project_notes_context(context.to_owned());

        assert!(trimmed.contains("manual before"));
        assert!(trimmed.contains("manual between"));
        assert!(trimmed.contains("manual after"));
        assert!(trimmed.contains("LATEST_INDEX src/new"));
        assert!(!trimmed.contains("OLD_INDEX src/old"));
        assert!(!trimmed.contains("LEGACY_STALE src/stale"));
        let stats = model_pool_index_note_stats(&trimmed);
        assert_eq!(stats.block_count, 1);
        assert_eq!(stats.delimited_blocks, 1);
        assert_eq!(stats.legacy_undelimited_blocks, 0);
        assert_eq!(stats.active, ModelPoolIndexNoteActive::LatestDelimited);
    }

    #[test]
    fn trim_project_notes_context_sanitizes_legacy_control_chars() {
        let context = concat!(
            "manual\x1b[31m note\n",
            "model_pool_index:\n",
            "source_prompt: repo map\n",
            "selected_role: index\n",
            "selected_base_url: http://127.0.0.1:8687\n",
            "answer:\n",
            "LATEST_INDEX src\x07/model_service\r\n",
            "model_pool_index_end:\n"
        );

        let trimmed = trim_project_notes_context(context.to_owned());

        assert!(trimmed.contains("manual [31m note"));
        assert!(trimmed.contains("LATEST_INDEX src /model_service\n"));
        assert!(!trimmed.contains('\x1b'));
        assert!(!trimmed.contains('\x07'));
        assert!(!trimmed.contains('\r'));
        assert!(first_disallowed_project_notes_control_char(&trimmed).is_none());
    }

    #[test]
    fn trim_project_notes_context_keeps_oversized_latest_index_queryable() {
        let context = format!(
            "{}\nmodel_pool_index:\nsource_prompt: repo map\nselected_role: index\nselected_base_url: http://127.0.0.1:8687\nanswer:\nOVERSIZED_INDEX {}\nmodel_pool_index_end:\n",
            "manual note\n".repeat(MAX_PROJECT_NOTES_CONTEXT_CHARS),
            "x".repeat(MAX_PROJECT_NOTES_CONTEXT_CHARS)
        );

        let trimmed = trim_project_notes_context(context);

        assert_eq!(trimmed.chars().count(), MAX_PROJECT_NOTES_CONTEXT_CHARS);
        assert!(trimmed.starts_with(MODEL_POOL_INDEX_NOTE_MARKER));
        assert!(trimmed.contains("OVERSIZED_INDEX"));
        assert!(trimmed.contains("[model_pool_index block truncated]"));
        assert!(trimmed.ends_with(MODEL_POOL_INDEX_NOTE_END_MARKER));
    }

    #[test]
    fn clears_model_pool_index_notes_from_store() {
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let root = std::env::temp_dir().join(format!(
            "smartsteam_index_notes_test_{}_{}",
            millis,
            std::process::id()
        ));
        let path = root.join("project_notes.md");
        let store = ProjectNotesStore::open(&path);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(
            &path,
            concat!(
                "manual\n",
                "model_pool_index:\n",
                "answer:\n",
                "src/model_service\n",
                "model_pool_index_end:\n",
                "manual after\n",
                "model_pool_index:\n",
                "legacy stale\n"
            ),
        )
        .unwrap();
        let shown = store.model_pool_index_notes().unwrap();
        assert!(shown.contains("model_pool_index_notes=2"));
        assert!(shown.contains("legacy_undelimited=1"));

        let cleared = store.clear_model_pool_index_notes().unwrap();
        assert!(cleared.contains("removed=2"));
        assert!(cleared.contains("legacy_undelimited=1"));
        let content = store.read().unwrap();
        assert!(content.contains("manual"));
        assert!(content.contains("manual after"));
        assert!(!content.contains("src/model_service"));
        assert!(!content.contains("legacy stale"));
        assert_eq!(
            model_pool_index_note_stats(&content).active,
            ModelPoolIndexNoteActive::None
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn clear_model_pool_index_notes_sanitizes_remaining_project_notes_on_disk() {
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let root = std::env::temp_dir().join(format!(
            "smartsteam_index_notes_sanitize_test_{}_{}",
            millis,
            std::process::id()
        ));
        let path = root.join("project_notes.md");
        let store = ProjectNotesStore::open(&path);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(
            &path,
            concat!(
                "manual\x1b[31m before\r\n",
                "model_pool_index:\n",
                "answer:\n",
                "src/model_service\n",
                "model_pool_index_end:\n",
                "manual after\x07\n"
            ),
        )
        .unwrap();

        let cleared = store.clear_model_pool_index_notes().unwrap();
        let content = store.read().unwrap();

        assert!(cleared.contains("removed=1"));
        assert!(content.contains("manual [31m before\n"));
        assert!(content.contains("manual after "));
        assert!(!content.contains("src/model_service"));
        assert!(!content.contains('\x1b'));
        assert!(!content.contains('\x07'));
        assert!(!content.contains('\r'));
        assert!(first_disallowed_project_notes_control_char(&content).is_none());

        let _ = fs::remove_dir_all(root);
    }
}
