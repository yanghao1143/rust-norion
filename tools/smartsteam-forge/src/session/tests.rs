use super::*;

struct MockProvider {
    events: Vec<StreamEvent>,
}

impl StreamProvider for MockProvider {
    fn stream(
        &self,
        _request: &StreamRequest,
        on_event: &mut dyn FnMut(StreamEvent) -> Result<(), String>,
    ) -> Result<(), String> {
        for event in self.events.clone() {
            on_event(event)?;
        }
        Ok(())
    }
}

#[test]
fn stream_prompt_updates_memory_from_final_answer() {
    let provider = MockProvider {
        events: vec![
            StreamEvent::Delta("草稿".to_owned()),
            StreamEvent::Final("{\"answer\":\"最终答案\"}".to_owned()),
            StreamEvent::Done,
        ],
    };
    let mut session = ForgeSession::default();
    let mut seen = Vec::new();

    let answer = session
        .stream_prompt(&provider, "你好", &mut |event| {
            seen.push(event);
            Ok(())
        })
        .unwrap();

    assert_eq!(answer.streamed_text, "草稿");
    assert_eq!(answer.assistant_message, "最终答案");
    assert_eq!(session.memory().messages().len(), 2);
    assert_eq!(session.memory().messages()[0].content, "你好");
    assert_eq!(session.memory().messages()[1].content, "最终答案");
    assert!(matches!(seen.last(), Some(StreamEvent::Done)));
}

#[test]
fn stream_prompt_ignores_transient_events_for_memory_answer() {
    let provider = MockProvider {
        events: vec![
            StreamEvent::Stage("generate".to_owned()),
            StreamEvent::Meta("runtime_tokens=12".to_owned()),
            StreamEvent::Status("backend still running".to_owned()),
            StreamEvent::Heartbeat("backend heartbeat".to_owned()),
            StreamEvent::Message {
                event: "trace".to_owned(),
                data: "debug only".to_owned(),
            },
            StreamEvent::Delta("visible answer".to_owned()),
            StreamEvent::Done,
        ],
    };
    let mut session = ForgeSession::default();
    let mut seen = Vec::new();

    let answer = session
        .stream_prompt(&provider, "你好", &mut |event| {
            seen.push(event);
            Ok(())
        })
        .unwrap();

    assert_eq!(answer.streamed_text, "visible answer");
    assert_eq!(answer.assistant_message, "visible answer");
    assert_eq!(seen.len(), 7);
    assert_eq!(session.memory().messages().len(), 2);
    assert_eq!(session.memory().messages()[0].content, "你好");
    assert_eq!(session.memory().messages()[1].content, "visible answer");
    for message in session.memory().messages() {
        assert!(!message.content.contains("runtime_tokens=12"));
        assert!(!message.content.contains("backend still running"));
        assert!(!message.content.contains("backend heartbeat"));
        assert!(!message.content.contains("debug only"));
    }
}

#[test]
fn stream_prompt_rejects_soft_truncated_stream_without_updating_memory() {
    let provider = MockProvider {
        events: vec![StreamEvent::Delta("半截答案".to_owned())],
    };
    let mut session = ForgeSession::default();
    let mut seen = Vec::new();

    let error = session
        .stream_prompt(&provider, "你好", &mut |event| {
            seen.push(event);
            Ok(())
        })
        .unwrap_err();

    assert!(error.contains("provider stream ended before done event"));
    assert!(error.contains("partial answer discarded"));
    assert_eq!(seen, vec![StreamEvent::Delta("半截答案".to_owned())]);
    assert!(session.memory().messages().is_empty());
}

#[test]
fn stream_prompt_rejects_error_event_without_updating_memory() {
    let provider = MockProvider {
        events: vec![StreamEvent::Error("backend failed".to_owned())],
    };
    let mut session = ForgeSession::default();

    let error = session
        .stream_prompt(&provider, "你好", &mut |_| Ok(()))
        .unwrap_err();

    assert!(error.contains("backend failed"));
    assert!(session.memory().messages().is_empty());
}

#[test]
fn stream_prompt_discards_partial_answer_for_error_event_after_delta() {
    let provider = MockProvider {
        events: vec![
            StreamEvent::Delta("半截答案".to_owned()),
            StreamEvent::Error("backend failed".to_owned()),
        ],
    };
    let mut session = ForgeSession::default();

    let error = session
        .stream_prompt(&provider, "你好", &mut |_| Ok(()))
        .unwrap_err();

    assert!(error.contains("backend failed"));
    assert!(error.contains("partial answer discarded"));
    assert!(session.memory().messages().is_empty());
}

#[test]
fn chat_request_carries_previous_turn_context() {
    let mut session = ForgeSession::default();
    let provider = MockProvider {
        events: vec![
            StreamEvent::Delta("first answer".to_owned()),
            StreamEvent::Done,
        ],
    };

    session
        .stream_prompt(&provider, "first question", &mut |_| Ok(()))
        .unwrap();
    let request = session.build_request("second question");

    assert!(
        request
            .messages
            .iter()
            .any(|message| message.role == "user" && message.content == "first question")
    );
    assert!(
        request
            .messages
            .iter()
            .any(|message| message.role == "assistant" && message.content == "first answer")
    );
    assert_eq!(request.messages.last().unwrap().content, "second question");
}

#[test]
fn context_preview_shows_short_history_and_pinned_context() {
    let mut session = ForgeSession::default();
    session.memory_mut().push_user("first question");
    session.memory_mut().push_assistant("first answer");
    session.set_project_notes_context("Pinned business rule");
    session.set_summary_context("Resumed session summary");

    let preview = session.context_preview();

    assert!(preview.contains("short_history_messages=2"));
    assert!(preview.contains("context_budget:"));
    assert!(preview.contains("messages_sent=5"));
    assert!(preview.contains("history_kept=2"));
    assert!(preview.contains("history_dropped=0"));
    assert!(preview.contains("pinned_messages=2"));
    assert!(preview.contains("model_pool_index_context: none"));
    assert!(preview.contains("1. user [sent_next_request]: first question"));
    assert!(preview.contains("2. assistant [sent_next_request]: first answer"));
    assert!(preview.contains("project_notes_preview: Pinned business rule"));
    assert!(preview.contains("summary_context_preview: Resumed session summary"));
}

#[test]
fn context_preview_reports_model_pool_index_note_stats() {
    let mut session = ForgeSession::default();
    session.set_project_notes_context(concat!(
        "manual note\n",
        "model_pool_index:\n",
        "source_prompt: repo map\n",
        "selected_role: index\n",
        "selected_base_url: http://127.0.0.1:8687\n",
        "answer:\n",
        "src/model_service handles model pool routing\n",
        "model_pool_index_end:\n"
    ));

    let preview = session.context_preview();
    let budget = session.context_budget_summary();
    let stats = session.model_pool_index_note_stats();

    assert_eq!(stats.block_count, 1);
    assert_eq!(stats.delimited_blocks, 1);
    assert_eq!(stats.legacy_undelimited_blocks, 0);
    assert_eq!(stats.trusted_blocks, 1);
    assert_eq!(
        stats.context_active,
        ModelPoolIndexNoteContextActive::LatestTrustedDelimited
    );
    assert!(stats.total_chars > 0);
    assert!(preview.contains("model_pool_index_context: blocks=1 delimited=1"));
    assert!(preview.contains("legacy_undelimited=0"));
    assert!(preview.contains("active=latest_delimited"));
    assert!(preview.contains("trusted=1"));
    assert!(preview.contains("context_active=latest_trusted_delimited"));
    assert!(preview.contains("project_notes_preview: manual note"));
    assert!(!preview.contains("source_prompt: repo map"));
    assert!(!preview.contains("src/model_service handles model pool routing"));
    assert!(budget.contains("model_pool_index_notes=1"));
    assert!(budget.contains("model_pool_index_active=latest_delimited"));
    assert!(budget.contains("model_pool_index_trusted=1"));
    assert!(budget.contains("model_pool_index_context_active=latest_trusted_delimited"));
    assert!(budget.contains("model_pool_index_legacy_undelimited=0"));
}

#[test]
fn context_preview_ignores_legacy_index_context_when_no_trusted_index_exists() {
    let mut session = ForgeSession::default();
    session.set_project_notes_context(concat!(
        "manual note\n",
        "model_pool_index:\n",
        "legacy-only src/old\n"
    ));

    let preview = session.context_preview();
    let budget = session.context_budget_summary();

    assert!(preview.contains("model_pool_index_context: none"));
    assert!(preview.contains("project_notes_preview: manual note"));
    assert!(budget.contains("model_pool_index_active=none"));
    assert!(budget.contains("model_pool_index_trusted=0"));
    assert!(budget.contains("model_pool_index_context_active=none"));
    assert!(!preview.contains("legacy-only src/old"));
}

#[test]
fn context_budget_reports_bounded_next_request_history() {
    let mut session = ForgeSession::default();
    session.set_max_context_messages(10);
    for index in 0..12 {
        session.memory_mut().push_user(format!("history {index}"));
    }

    let budget = session.context_budget_summary();
    let request = session.build_request("next prompt");

    assert_eq!(request.messages.len(), 10);
    assert!(budget.contains("messages_sent=10"));
    assert!(budget.contains("history_kept=9"));
    assert!(budget.contains("history_dropped=1"));
    assert!(budget.contains("max_context_messages=10"));
    assert_eq!(request.messages.first().unwrap().content, "history 3");
    assert_eq!(request.messages.last().unwrap().content, "next prompt");
}

#[test]
fn default_session_uses_longer_context_and_explicit_token_budget() {
    let session = ForgeSession::default();
    let request = session.build_request("next prompt");

    assert_eq!(session.settings().max_context_messages, 64);
    assert_eq!(request.max_tokens, Some(262_144));
    assert!(session.settings_summary().contains("max_tokens=262144"));
}

#[test]
fn set_max_context_messages_trims_history_and_updates_budget() {
    let mut session = ForgeSession::default();
    for index in 0..8 {
        session.memory_mut().push_user(format!("history {index}"));
    }

    session.set_max_context_messages(4);
    let request = session.build_request("next prompt");
    let budget = session.context_budget_summary();

    assert_eq!(session.settings().max_context_messages, 4);
    assert_eq!(session.memory().messages().len(), 4);
    assert_eq!(session.memory().messages()[0].content, "history 4");
    assert_eq!(request.messages.len(), 4);
    assert_eq!(request.messages[0].content, "history 5");
    assert_eq!(request.messages.last().unwrap().content, "next prompt");
    assert!(budget.contains("max_context_messages=4"));
    assert!(budget.contains("history_kept=3"));
}

#[test]
fn set_max_context_messages_clamps_to_minimum_prompt_plus_one_history() {
    let mut session = ForgeSession::default();
    for index in 0..4 {
        session.memory_mut().push_user(format!("history {index}"));
    }

    session.set_max_context_messages(1);
    let request = session.build_request("next prompt");
    let budget = session.context_budget_summary();

    assert_eq!(session.settings().max_context_messages, 2);
    assert_eq!(request.messages.len(), 2);
    assert_eq!(request.messages[0].content, "history 3");
    assert_eq!(request.messages[1].content, "next prompt");
    assert!(budget.contains("max_context_messages=2"));
    assert!(budget.contains("history_kept=1"));
    assert!(budget.contains("history_dropped=1"));
}

#[test]
fn build_request_carries_max_tokens() {
    let mut session = ForgeSession::default();
    session.set_max_tokens(Some(8192));

    let request = session.build_request("next prompt");

    assert_eq!(request.max_tokens, Some(8192));
    assert!(session.settings_summary().contains("max_tokens=8192"));
}

#[test]
fn stream_prompt_updates_memory_for_all_modes() {
    for endpoint in [
        StreamEndpoint::Chat,
        StreamEndpoint::Generate,
        StreamEndpoint::BusinessCycle,
    ] {
        let provider = MockProvider {
            events: vec![
                StreamEvent::Delta("草稿".to_owned()),
                StreamEvent::Final("{\"answer\":\"最终答案\"}".to_owned()),
                StreamEvent::Done,
            ],
        };
        let mut session = ForgeSession::default();
        session.set_endpoint(endpoint);

        let answer = session
            .stream_prompt(&provider, "上一轮问题", &mut |_| Ok(()))
            .unwrap();

        assert_eq!(answer.assistant_message, "最终答案");
        assert_eq!(session.memory().messages().len(), 2);
        assert_eq!(session.memory().messages()[0].role, "user");
        assert_eq!(session.memory().messages()[0].content, "上一轮问题");
        assert_eq!(session.memory().messages()[1].role, "assistant");
        assert_eq!(session.memory().messages()[1].content, "最终答案");
    }
}

#[test]
fn prompt_only_modes_embed_recent_short_term_history() {
    for endpoint in [StreamEndpoint::Generate, StreamEndpoint::BusinessCycle] {
        let provider = MockProvider {
            events: vec![
                StreamEvent::Delta("first answer".to_owned()),
                StreamEvent::Done,
            ],
        };
        let mut session = ForgeSession::default();
        session.set_endpoint(endpoint);

        session
            .stream_prompt(&provider, "first question", &mut |_| Ok(()))
            .unwrap();
        let request = session.build_request("second question");
        let budget = session.context_budget_summary();

        assert_eq!(request.endpoint, endpoint);
        assert!(request.prompt.contains("user: first question"));
        assert!(request.prompt.contains("assistant: first answer"));
        assert!(request.prompt.contains("User prompt:\nsecond question"));
        assert!(budget.contains(&format!("mode={}", endpoint.label())));
        assert!(budget.contains("prompt_only_history_messages=2"));
    }
}

#[test]
fn loads_transcript_messages_into_short_context() {
    let mut session = ForgeSession::default();

    session.load_transcript_messages(vec![
        TranscriptMessage {
            role: "user".to_owned(),
            content: "old prompt".to_owned(),
        },
        TranscriptMessage {
            role: "assistant".to_owned(),
            content: "old answer".to_owned(),
        },
    ]);

    assert_eq!(session.memory().messages().len(), 2);
    assert_eq!(session.memory().messages()[1].content, "old answer");
}

#[test]
fn build_request_prepends_summary_context_for_chat_mode() {
    let mut session = ForgeSession::default();
    session.set_summary_context("previous session summary");

    let request = session.build_request("continue");

    assert_eq!(request.messages[0].role, "system");
    assert!(
        request.messages[0]
            .content
            .contains("previous session summary")
    );
    assert_eq!(request.messages.last().unwrap().content, "continue");
}

#[test]
fn build_request_prepends_project_notes_for_chat_mode() {
    let mut session = ForgeSession::default();
    session.set_project_notes_context("Prefer concise Chinese answers.");

    let request = session.build_request("continue");

    assert_eq!(request.messages[0].role, "system");
    assert!(
        request.messages[0]
            .content
            .contains("Prefer concise Chinese answers")
    );
    assert_eq!(request.messages.last().unwrap().content, "continue");
    assert!(session.settings_summary().contains("project_notes=loaded"));
}

#[test]
fn build_request_project_notes_keep_latest_complete_index_only() {
    let mut session = ForgeSession::default();
    session.set_project_notes_context(concat!(
        "manual note\n",
        "model_pool_index:\n",
        "source_prompt: old repo map\n",
        "selected_role: index\n",
        "selected_base_url: http://127.0.0.1:8687\n",
        "answer:\n",
        "OLD_INDEX src/old\n",
        "model_pool_index_end:\n",
        "model_pool_index:\n",
        "source_prompt: latest repo map\n",
        "selected_role: index\n",
        "selected_base_url: http://127.0.0.1:8687\n",
        "answer:\n",
        "LATEST_INDEX src/new\n",
        "model_pool_index_end:\n",
        "model_pool_index:\n",
        "LEGACY_STALE src/stale\n"
    ));

    let request = session.build_request("continue");
    let system_context = &request.messages[0].content;

    assert!(system_context.contains("manual note"));
    assert!(system_context.contains("LATEST_INDEX src/new"));
    assert!(!system_context.contains("OLD_INDEX src/old"));
    assert!(!system_context.contains("LEGACY_STALE src/stale"));
    let stats = session.model_pool_index_note_stats();
    assert_eq!(stats.block_count, 1);
    assert_eq!(stats.delimited_blocks, 1);
    assert_eq!(stats.legacy_undelimited_blocks, 0);
    assert_eq!(stats.active, ModelPoolIndexNoteActive::LatestDelimited);
}

#[test]
fn build_request_prefixes_project_notes_for_prompt_only_modes() {
    let mut session = ForgeSession::default();
    session.set_endpoint(StreamEndpoint::BusinessCycle);
    session.set_project_notes_context("Business gate must pass.");

    let request = session.build_request("run gate");

    assert!(request.prompt.contains("Business gate must pass"));
    assert!(request.prompt.contains("User prompt:\nrun gate"));
}

#[test]
fn clear_removes_summary_context() {
    let mut session = ForgeSession::default();
    session.set_summary_context("summary");
    session.set_project_notes_context("notes");

    session.clear();

    assert_eq!(session.summary_context_chars(), 0);
    assert_eq!(session.project_notes_context_chars(), 5);
    assert!(session.settings_summary().contains("summary_context=none"));
    assert!(session.settings_summary().contains("project_notes=loaded"));
}

#[test]
fn business_cycle_request_uses_rust_check_settings() {
    let mut session = ForgeSession::default();
    session.set_endpoint(StreamEndpoint::BusinessCycle);
    session.set_rust_check_code("pub fn ok() {}");
    session.set_rust_check_edition("2024");
    session.set_rust_check_case(Some("forge-smoke".to_owned()));

    let request = session.build_request("检查一下");

    assert_eq!(request.rust_check_code.as_deref(), Some("pub fn ok() {}"));
    assert_eq!(request.rust_check_edition, "2024");
    assert_eq!(request.rust_check_case.as_deref(), Some("forge-smoke"));
    assert!(session.settings_summary().contains("rust_check=on"));
}
