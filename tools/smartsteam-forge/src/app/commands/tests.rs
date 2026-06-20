use super::*;
use smartsteam_forge::{SessionFilter, StreamEndpoint};

#[test]
fn parses_reserved_slash_commands() {
    assert_eq!(parse_slash_command("/help"), Some(SlashCommand::Help));
    assert_eq!(parse_slash_command("/clear"), Some(SlashCommand::Clear));
    assert_eq!(parse_slash_command("/new"), Some(SlashCommand::New));
    assert_eq!(parse_slash_command("/status"), Some(SlashCommand::Status));
    assert_eq!(
        parse_slash_command("/strict-status"),
        Some(SlashCommand::EvolutionStrictSummary(None))
    );
    assert_eq!(
        parse_slash_command("/strict-summary target\\evolution\\strict-status-summary.json"),
        Some(SlashCommand::EvolutionStrictSummary(Some(
            "target\\evolution\\strict-status-summary.json".to_owned()
        )))
    );
    assert_eq!(parse_slash_command("/ready"), Some(SlashCommand::Ready));
    assert_eq!(parse_slash_command("/hygiene"), Some(SlashCommand::Hygiene));
    assert_eq!(
        parse_slash_command("/hygiene dry-run 7"),
        Some(SlashCommand::HygieneQuarantineDryRun(7))
    );
    assert_eq!(
        parse_slash_command("/repair"),
        Some(SlashCommand::ExperienceRepairDryRun(20))
    );
    assert_eq!(
        parse_slash_command("/repair dry-run 9"),
        Some(SlashCommand::ExperienceRepairDryRun(9))
    );
    assert_eq!(
        parse_slash_command("/audit"),
        Some(SlashCommand::ExperienceCleanupAudit(20))
    );
    assert_eq!(
        parse_slash_command("/audit 7"),
        Some(SlashCommand::ExperienceCleanupAudit(7))
    );
    assert_eq!(
        parse_slash_command("/audit report 9"),
        Some(SlashCommand::ExperienceCleanupAudit(9))
    );
    assert_eq!(
        parse_slash_command("/retrieve 帮我用rust输出for循环"),
        Some(SlashCommand::RetrievalPreview {
            prompt: "帮我用rust输出for循环".to_owned(),
            limit: 5,
        })
    );
    assert_eq!(
        parse_slash_command("/retrieve 3 帮我用rust输出for循环"),
        Some(SlashCommand::RetrievalPreview {
            prompt: "帮我用rust输出for循环".to_owned(),
            limit: 3,
        })
    );
    assert_eq!(
        parse_slash_command("/pool-status"),
        Some(SlashCommand::ModelPoolStatus)
    );
    assert_eq!(
        parse_slash_command("/pool-manifest"),
        Some(SlashCommand::ModelPoolManifest)
    );
    assert_eq!(
        parse_slash_command("/pool-advice"),
        Some(SlashCommand::ModelPoolAdvice)
    );
    assert_eq!(
        parse_slash_command("/pool-smoke"),
        Some(SlashCommand::ModelPoolSmoke)
    );
    assert_eq!(
        parse_slash_command("/pool"),
        Some(SlashCommand::ModelPoolStatus)
    );
    assert_eq!(
        parse_slash_command("/pool advice"),
        Some(SlashCommand::ModelPoolAdvice)
    );
    assert_eq!(
        parse_slash_command("/pool manifest"),
        Some(SlashCommand::ModelPoolManifest)
    );
    assert_eq!(
        parse_slash_command("/pool smoke"),
        Some(SlashCommand::ModelPoolSmoke)
    );
    assert_eq!(
        parse_slash_command("/pool-watch"),
        Some(SlashCommand::ModelPoolWatch(ModelPoolWatchCommand {
            interval_secs: 5,
            max_iterations: None,
            enabled: true,
        }))
    );
    assert_eq!(
        parse_slash_command("/pool-watch 2 3"),
        Some(SlashCommand::ModelPoolWatch(ModelPoolWatchCommand {
            interval_secs: 2,
            max_iterations: Some(3),
            enabled: true,
        }))
    );
    assert_eq!(
        parse_slash_command("/pool watch off"),
        Some(SlashCommand::ModelPoolWatch(ModelPoolWatchCommand {
            interval_secs: 5,
            max_iterations: None,
            enabled: false,
        }))
    );
    assert_eq!(
        parse_slash_command("/pool-route review"),
        Some(SlashCommand::ModelPoolRoute("review".to_owned()))
    );
    assert_eq!(
        parse_slash_command("/pool route test"),
        Some(SlashCommand::ModelPoolRoute("test-gate".to_owned()))
    );
    assert_eq!(
        parse_slash_command("/pool-route index"),
        Some(SlashCommand::ModelPoolRoute("index".to_owned()))
    );
    assert_eq!(
        parse_slash_command("/pool-route spare"),
        Some(SlashCommand::ModelPoolRoute("index".to_owned()))
    );
    assert_eq!(
        parse_slash_command("/pool-call review 看看这个补丁有什么风险"),
        Some(SlashCommand::ModelPoolCall {
            task_kind: "review".to_owned(),
            prompt: "看看这个补丁有什么风险".to_owned(),
        })
    );
    assert_eq!(
        parse_slash_command("/pool call summary 总结日志"),
        Some(SlashCommand::ModelPoolCall {
            task_kind: "summary".to_owned(),
            prompt: "总结日志".to_owned(),
        })
    );
    assert_eq!(
        parse_slash_command("/pool call repo-index 更新仓库索引"),
        Some(SlashCommand::ModelPoolCall {
            task_kind: "index".to_owned(),
            prompt: "更新仓库索引".to_owned(),
        })
    );
    assert_eq!(
        parse_slash_command("/pool-call Index 更新仓库索引"),
        Some(SlashCommand::ModelPoolCall {
            task_kind: "index".to_owned(),
            prompt: "更新仓库索引".to_owned(),
        })
    );
    assert_eq!(
        parse_slash_command("/pool-call spare 更新仓库索引"),
        Some(SlashCommand::ModelPoolCall {
            task_kind: "index".to_owned(),
            prompt: "更新仓库索引".to_owned(),
        })
    );
    assert_eq!(parse_slash_command("/doctor"), Some(SlashCommand::Doctor));
    assert_eq!(parse_slash_command("/diagnose"), Some(SlashCommand::Doctor));
    assert_eq!(parse_slash_command("/cancel"), Some(SlashCommand::Cancel));
    assert_eq!(parse_slash_command("/abort"), Some(SlashCommand::Cancel));
    assert_eq!(
        parse_slash_command("/guard on"),
        Some(SlashCommand::Guard(true))
    );
    assert_eq!(
        parse_slash_command("/safe-device on"),
        Some(SlashCommand::SafeDeviceGuard(true))
    );
    assert_eq!(
        parse_slash_command("/device-guard off"),
        Some(SlashCommand::SafeDeviceGuard(false))
    );
    assert_eq!(parse_slash_command("/context"), Some(SlashCommand::Context));
    assert_eq!(parse_slash_command("/ctx"), Some(SlashCommand::Context));
    assert_eq!(
        parse_slash_command("/require-health off"),
        Some(SlashCommand::Guard(false))
    );
    assert_eq!(
        parse_slash_command("/sessions"),
        Some(SlashCommand::Sessions {
            filter: SessionFilter::All,
            limit: 50,
        })
    );
    assert_eq!(
        parse_slash_command("/sessions failed"),
        Some(SlashCommand::Sessions {
            filter: SessionFilter::Failed,
            limit: 50,
        })
    );
    assert_eq!(
        parse_slash_command("/history passed"),
        Some(SlashCommand::Sessions {
            filter: SessionFilter::Passed,
            limit: 50,
        })
    );
    assert_eq!(
        parse_slash_command("/sessions 100"),
        Some(SlashCommand::Sessions {
            filter: SessionFilter::All,
            limit: 100,
        })
    );
    assert_eq!(
        parse_slash_command("/sessions failed 100"),
        Some(SlashCommand::Sessions {
            filter: SessionFilter::Failed,
            limit: 100,
        })
    );
    assert_eq!(
        parse_slash_command("/resume 2"),
        Some(SlashCommand::Resume("2".to_owned()))
    );
    assert_eq!(
        parse_slash_command("/summary 2"),
        Some(SlashCommand::Summary("2".to_owned()))
    );
    assert_eq!(parse_slash_command("/notes"), Some(SlashCommand::NotesShow));
    assert_eq!(
        parse_slash_command("/notes add keep context small"),
        Some(SlashCommand::NotesAdd("keep context small".to_owned()))
    );
    assert_eq!(
        parse_slash_command("/notes set pinned"),
        Some(SlashCommand::NotesSet("pinned".to_owned()))
    );
    assert_eq!(
        parse_slash_command("/notes clear"),
        Some(SlashCommand::NotesClear)
    );
    assert_eq!(
        parse_slash_command("/index-notes"),
        Some(SlashCommand::IndexNotesShow)
    );
    assert_eq!(
        parse_slash_command("/index-notes clear"),
        Some(SlashCommand::IndexNotesClear)
    );
    assert_eq!(
        parse_slash_command("/notes index"),
        Some(SlashCommand::IndexNotesShow)
    );
    assert_eq!(
        parse_slash_command("/notes index clear"),
        Some(SlashCommand::IndexNotesClear)
    );
    assert_eq!(parse_slash_command("/quit"), Some(SlashCommand::Quit));
    assert_eq!(
        parse_slash_command("/mode business-cycle"),
        Some(SlashCommand::Mode(StreamEndpoint::BusinessCycle))
    );
    assert_eq!(
        parse_slash_command("/output enhanced"),
        Some(SlashCommand::Output("enhanced".to_owned()))
    );
    assert_eq!(
        parse_slash_command("/context-window 6"),
        Some(SlashCommand::ContextWindow(6))
    );
    assert_eq!(
        parse_slash_command("/context-messages 7"),
        Some(SlashCommand::ContextWindow(7))
    );
    assert_eq!(
        parse_slash_command("/ctx-window 8"),
        Some(SlashCommand::ContextWindow(8))
    );
    assert_eq!(
        parse_slash_command("/max-tokens 8192"),
        Some(SlashCommand::MaxTokens(Some(8192)))
    );
    assert_eq!(
        parse_slash_command("/max-output-tokens default"),
        Some(SlashCommand::MaxTokens(None))
    );
    assert_eq!(
        parse_slash_command("/max-tokens auto"),
        Some(SlashCommand::MaxTokens(None))
    );
    assert_eq!(
        parse_slash_command("/max-tokens DEFAULT"),
        Some(SlashCommand::MaxTokens(None))
    );
}

#[test]
fn parses_rust_check_commands() {
    assert_eq!(
        parse_slash_command("/rust-check inline pub fn ok() {}"),
        Some(SlashCommand::RustCheckInline("pub fn ok() {}".to_owned()))
    );
    assert_eq!(
        parse_slash_command("/rust-check file src/lib.rs"),
        Some(SlashCommand::RustCheckFile("src/lib.rs".to_owned()))
    );
    assert_eq!(
        parse_slash_command("/rust-check edition 2024"),
        Some(SlashCommand::RustCheckEdition("2024".to_owned()))
    );
    assert_eq!(
        parse_slash_command("/rust-check case smoke"),
        Some(SlashCommand::RustCheckCase(Some("smoke".to_owned())))
    );
    assert_eq!(
        parse_slash_command("/rust-check off"),
        Some(SlashCommand::RustCheckClear)
    );
}

#[test]
fn rejects_invalid_context_window_commands() {
    assert_eq!(
        parse_slash_command("/context-window"),
        Some(SlashCommand::Unknown("context-window".to_owned()))
    );
    assert_eq!(
        parse_slash_command("/context-messages 0"),
        Some(SlashCommand::Unknown("context-messages".to_owned()))
    );
    assert_eq!(
        parse_slash_command("/ctx-window nope"),
        Some(SlashCommand::Unknown("ctx-window".to_owned()))
    );
    assert_eq!(
        parse_slash_command("/max-tokens 0"),
        Some(SlashCommand::Unknown("max-tokens".to_owned()))
    );
    assert_eq!(
        parse_slash_command("/pool-route launch"),
        Some(SlashCommand::Unknown("pool-route".to_owned()))
    );
}
