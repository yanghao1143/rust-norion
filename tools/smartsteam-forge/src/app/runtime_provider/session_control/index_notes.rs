const MAX_RETRIEVAL_INDEX_CONTEXT_CHARS: usize = 1800;

use smartsteam_forge::session::{latest_safe_model_pool_index_block, model_pool_index_note_stats};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app::runtime_provider) struct RetrievalPromptWithIndex {
    pub(in crate::app::runtime_provider) prompt: String,
    pub(in crate::app::runtime_provider) index_context: Option<String>,
    pub(in crate::app::runtime_provider) index_context_chars: usize,
    pub(in crate::app::runtime_provider) index_context_active_trusted: bool,
}

pub(in crate::app::runtime_provider) fn retrieval_prompt_with_index_context(
    prompt: &str,
    project_notes_context: Option<&str>,
) -> RetrievalPromptWithIndex {
    let Some(index_context) = project_notes_context.and_then(extract_model_pool_index_context)
    else {
        return RetrievalPromptWithIndex {
            prompt: prompt.to_owned(),
            index_context: None,
            index_context_chars: 0,
            index_context_active_trusted: false,
        };
    };
    let index_context_active_trusted = project_notes_context
        .map(model_pool_index_note_stats)
        .is_some_and(|stats| stats.active_trusted);
    let index_context = trim_chars(&index_context, MAX_RETRIEVAL_INDEX_CONTEXT_CHARS);
    RetrievalPromptWithIndex {
        index_context_chars: index_context.chars().count(),
        prompt: prompt.to_owned(),
        index_context: Some(index_context),
        index_context_active_trusted,
    }
}

pub(in crate::app::runtime_provider) fn append_index_context_query_status(
    summary: &mut String,
    index_context_chars: usize,
    active_trusted: bool,
) {
    if index_context_chars == 0 || summary.contains("index_context_query=") {
        return;
    }
    summary.push_str(&format!(
        "\nindex_context_query=used chars={index_context_chars} trusted=true active_trusted={active_trusted} context_active=latest_trusted_delimited"
    ));
}

fn extract_model_pool_index_context(project_notes_context: &str) -> Option<String> {
    latest_safe_model_pool_index_block(project_notes_context)
}

fn trim_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_owned();
    }
    let suffix = "\n[model_pool_index retrieval context truncated]";
    let keep_chars = max_chars.saturating_sub(suffix.chars().count());
    let mut out = value.chars().take(keep_chars).collect::<String>();
    out.push_str(suffix);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use smartsteam_forge::session::trim_project_notes_context;

    fn trusted_index(answer: &str) -> String {
        format!(
            concat!(
                "model_pool_index:\n",
                "source_prompt: repo map\n",
                "selected_role: index\n",
                "selected_base_url: http://127.0.0.1:8687\n",
                "answer:\n",
                "{}\n",
                "model_pool_index_end:\n"
            ),
            answer
        )
    }

    #[test]
    fn retrieval_prompt_uses_model_pool_index_context() {
        let notes = format!(
            "manual note\n{}",
            trusted_index("src/model_service handles model pool routing")
        );
        let prompt =
            retrieval_prompt_with_index_context("find model pool route code", Some(&notes));

        assert!(prompt.index_context_chars > 0);
        assert_eq!(prompt.prompt, "find model pool route code");
        assert!(prompt.index_context_active_trusted);
        assert!(
            prompt
                .index_context
                .as_deref()
                .unwrap()
                .contains("src/model_service handles model pool routing")
        );
        assert!(!prompt.prompt.contains("manual note"));
    }

    #[test]
    fn retrieval_prompt_ignores_plain_project_notes() {
        let prompt = retrieval_prompt_with_index_context(
            "find rust check notes",
            Some("plain project note without an index marker"),
        );

        assert_eq!(prompt.index_context_chars, 0);
        assert!(!prompt.index_context_active_trusted);
        assert_eq!(prompt.prompt, "find rust check notes");
        assert_eq!(prompt.index_context, None);
    }

    #[test]
    fn retrieval_prompt_marks_untrusted_visible_active_when_prior_trusted_index_is_used() {
        let notes = format!(
            "{}{}",
            trusted_index("old trusted src/model_service"),
            concat!(
                "model_pool_index:\n",
                "answer:\n",
                "untrusted latest src/manual\n",
                "model_pool_index_end:\n"
            )
        );
        let prompt = retrieval_prompt_with_index_context("find repo map", Some(&notes));

        let index_context = prompt.index_context.as_deref().unwrap();
        assert!(index_context.contains("old trusted src/model_service"));
        assert!(!index_context.contains("untrusted latest src/manual"));
        assert!(!prompt.index_context_active_trusted);
    }

    #[test]
    fn retrieval_prompt_truncates_large_index_context() {
        let notes = trusted_index(&"x".repeat(2400));
        let prompt = retrieval_prompt_with_index_context("find repo map", Some(&notes));

        assert_eq!(
            prompt.index_context_chars,
            MAX_RETRIEVAL_INDEX_CONTEXT_CHARS
        );
        assert!(prompt.index_context_active_trusted);
        assert!(
            prompt
                .index_context
                .as_deref()
                .unwrap()
                .contains("[model_pool_index retrieval context truncated]")
        );
    }

    #[test]
    fn retrieval_prompt_uses_latest_delimited_index_context() {
        let notes = format!(
            "{}manual note\n{}manual after\n",
            trusted_index("old src/old"),
            trusted_index("new src/model_service")
        );
        let prompt = retrieval_prompt_with_index_context("find repo map", Some(&notes));

        let index_context = prompt.index_context.as_deref().unwrap();
        assert!(index_context.contains("new src/model_service"));
        assert!(!index_context.contains("old src/old"));
        assert!(!index_context.contains("manual after"));
    }

    #[test]
    fn retrieval_prompt_prefers_latest_complete_index_over_trailing_legacy() {
        let notes = format!(
            "{}manual note\nmodel_pool_index:\nlegacy stale src/old\n",
            trusted_index("complete src/model_service")
        );
        let prompt = retrieval_prompt_with_index_context("find repo map", Some(&notes));

        let index_context = prompt.index_context.as_deref().unwrap();
        assert!(index_context.contains("complete src/model_service"));
        assert!(!index_context.contains("legacy stale src/old"));
    }

    #[test]
    fn retrieval_prompt_uses_latest_index_after_project_notes_trim() {
        let notes = format!(
            "{}{}{}",
            trusted_index("OLD_INDEX src/old"),
            "manual note\n".repeat(4000),
            trusted_index("LATEST_INDEX src/model_service")
        );
        let trimmed_notes = trim_project_notes_context(notes);

        let prompt = retrieval_prompt_with_index_context("find repo map", Some(&trimmed_notes));
        let index_context = prompt.index_context.as_deref().unwrap();

        assert!(index_context.contains("LATEST_INDEX src/model_service"));
        assert!(!index_context.contains("OLD_INDEX src/old"));
    }

    #[test]
    fn retrieval_prompt_sanitizes_legacy_control_chars_in_index_context() {
        let notes = trusted_index("src\x1b[2J/model_service\x08 handles\r\nrouting");
        let prompt = retrieval_prompt_with_index_context("find repo map", Some(&notes));

        let index_context = prompt.index_context.as_deref().unwrap();
        assert!(index_context.contains("src [2J/model_service  handles\nrouting"));
        assert!(!index_context.contains('\x1b'));
        assert!(!index_context.contains('\x08'));
        assert!(!index_context.contains('\r'));
    }

    #[test]
    fn appends_index_context_query_status_only_when_used() {
        let mut summary = "Noiron experience retrieval preview".to_owned();
        append_index_context_query_status(&mut summary, 0, false);
        assert_eq!(summary, "Noiron experience retrieval preview");

        append_index_context_query_status(&mut summary, 42, true);
        assert!(summary.contains(
            "index_context_query=used chars=42 trusted=true active_trusted=true context_active=latest_trusted_delimited"
        ));
    }

    #[test]
    fn appends_legacy_index_context_query_status_for_old_backend_summary() {
        let mut summary = "Noiron experience retrieval preview\nrequested_limit=5".to_owned();

        append_index_context_query_status(&mut summary, 64, false);

        assert!(summary.contains(
            "index_context_query=used chars=64 trusted=true active_trusted=false context_active=latest_trusted_delimited"
        ));
    }

    #[test]
    fn appends_trusted_query_status_when_backend_reports_index_context() {
        let mut summary =
            "Noiron experience retrieval preview\nindex_context_used=true\nindex_context_chars=42"
                .to_owned();

        append_index_context_query_status(&mut summary, 42, true);

        assert!(summary.contains("index_context_used=true"));
        assert!(summary.contains(
            "index_context_query=used chars=42 trusted=true active_trusted=true context_active=latest_trusted_delimited"
        ));
    }

    #[test]
    fn does_not_duplicate_existing_index_context_query_status() {
        let mut summary = concat!(
            "Noiron experience retrieval preview\n",
            "index_context_query=used chars=42 trusted=true active_trusted=true context_active=latest_trusted_delimited"
        )
        .to_owned();

        append_index_context_query_status(&mut summary, 64, false);

        assert_eq!(summary.matches("index_context_query=used").count(), 1);
        assert!(!summary.contains("chars=64"));
    }
}
