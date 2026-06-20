use smartsteam_forge::{SessionFilter, StreamEndpoint};

use super::{ModelPoolWatchCommand, SlashCommand};

const DEFAULT_MODEL_POOL_WATCH_INTERVAL_SECS: u64 = 5;

pub fn parse_slash_command(input: &str) -> Option<SlashCommand> {
    let command = input.strip_prefix('/')?;
    let (name, rest) = command
        .split_once(char::is_whitespace)
        .map(|(name, rest)| (name, rest.trim()))
        .unwrap_or((command, ""));
    Some(match name {
        "help" => SlashCommand::Help,
        "clear" => SlashCommand::Clear,
        "new" | "reset" => SlashCommand::New,
        "status" => SlashCommand::Status,
        "strict-status"
        | "strict-summary"
        | "evolution-strict-summary"
        | "daemon-strict-summary" => SlashCommand::EvolutionStrictSummary(optional_path(rest)),
        "ready" | "readiness" | "preflight" => SlashCommand::Ready,
        "hygiene" | "memory-hygiene" | "experience-hygiene" => {
            parse_hygiene_command(rest).unwrap_or_else(|| SlashCommand::Unknown(name.to_owned()))
        }
        "repair" | "experience-repair" => {
            parse_repair_command(rest).unwrap_or_else(|| SlashCommand::Unknown(name.to_owned()))
        }
        "audit" | "cleanup-audit" | "experience-cleanup-audit" => {
            parse_audit_command(rest).unwrap_or_else(|| SlashCommand::Unknown(name.to_owned()))
        }
        "retrieve" | "retrieval" | "experience-retrieval" => {
            parse_retrieval_command(rest).unwrap_or_else(|| SlashCommand::Unknown(name.to_owned()))
        }
        "pool-status" | "model-pool-status" => SlashCommand::ModelPoolStatus,
        "pool-manifest" | "model-pool-manifest" | "apple-pool-manifest" => {
            SlashCommand::ModelPoolManifest
        }
        "pool-advice" | "model-pool-advice" | "apple-pool-advice" | "pool-capacity" => {
            SlashCommand::ModelPoolAdvice
        }
        "pool-smoke" | "model-pool-smoke" | "apple-pool-smoke" | "smoke-pool" => {
            SlashCommand::ModelPoolSmoke
        }
        "pool-watch" | "model-pool-watch" => {
            parse_pool_watch_command(rest).unwrap_or_else(|| SlashCommand::Unknown(name.to_owned()))
        }
        "pool-route" | "model-pool-route" | "route-plan" => match parse_pool_task_kind(rest) {
            Some(task_kind) => SlashCommand::ModelPoolRoute(task_kind),
            None => SlashCommand::Unknown(name.to_owned()),
        },
        "pool-call" | "model-pool-call" => {
            parse_pool_call_command(rest).unwrap_or_else(|| SlashCommand::Unknown(name.to_owned()))
        }
        "pool" | "model-pool" => {
            parse_pool_command(rest).unwrap_or_else(|| SlashCommand::Unknown(name.to_owned()))
        }
        "doctor" | "diagnose" | "diagnostic" => SlashCommand::Doctor,
        "cancel" | "abort" | "stop" => SlashCommand::Cancel,
        "guard" | "require-health" => match parse_bool(rest) {
            Some(enabled) => SlashCommand::Guard(enabled),
            None => SlashCommand::Unknown(name.to_owned()),
        },
        "safe-device" | "safe" | "device-guard" => match parse_bool(rest) {
            Some(enabled) => SlashCommand::SafeDeviceGuard(enabled),
            None => SlashCommand::Unknown(name.to_owned()),
        },
        "show" => SlashCommand::Show,
        "context" | "ctx" => SlashCommand::Context,
        "sessions" | "history" => match parse_sessions_command(rest) {
            Some((filter, limit)) => SlashCommand::Sessions { filter, limit },
            None => SlashCommand::Unknown(name.to_owned()),
        },
        "resume" => SlashCommand::Resume(rest.to_owned()),
        "summary" | "summarize" => SlashCommand::Summary(rest.to_owned()),
        "notes" | "note" | "project-notes" => {
            parse_notes_command(rest).unwrap_or_else(|| SlashCommand::Unknown(name.to_owned()))
        }
        "index-notes" | "pool-index" | "model-pool-index" => parse_index_notes_command(rest)
            .unwrap_or_else(|| SlashCommand::Unknown(name.to_owned())),
        "mode" | "endpoint" => match parse_endpoint(rest) {
            Some(endpoint) => SlashCommand::Mode(endpoint),
            None => SlashCommand::Unknown(name.to_owned()),
        },
        "output" => match parse_output(rest) {
            Some(output) => SlashCommand::Output(output),
            None => SlashCommand::Unknown(name.to_owned()),
        },
        "profile" => match parse_profile(rest) {
            Some(profile) => SlashCommand::Profile(profile),
            None => SlashCommand::Unknown(name.to_owned()),
        },
        "feedback" => match parse_feedback(rest) {
            Some(amount) => SlashCommand::Feedback(amount),
            None => SlashCommand::Unknown(name.to_owned()),
        },
        "self" | "self-improve" => match parse_bool(rest) {
            Some(enabled) => SlashCommand::SelfImprove(enabled),
            None => SlashCommand::Unknown(name.to_owned()),
        },
        "context-window" | "context-messages" | "ctx-window" => match parse_positive_usize(rest) {
            Some(max_messages) => SlashCommand::ContextWindow(max_messages),
            None => SlashCommand::Unknown(name.to_owned()),
        },
        "max-tokens" | "max-output-tokens" => match parse_optional_max_tokens(rest) {
            Some(max_tokens) => SlashCommand::MaxTokens(max_tokens),
            None => SlashCommand::Unknown(name.to_owned()),
        },
        "rust-check" | "rustcheck" => {
            parse_rust_check_command(rest).unwrap_or_else(|| SlashCommand::Unknown(name.to_owned()))
        }
        "quit" | "exit" => SlashCommand::Quit,
        other => SlashCommand::Unknown(other.to_string()),
    })
}

fn optional_path(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn parse_pool_command(rest: &str) -> Option<SlashCommand> {
    let rest = rest.trim();
    if rest.is_empty() || matches!(rest, "status" | "workers") {
        return Some(SlashCommand::ModelPoolStatus);
    }
    let (verb, value) = rest
        .split_once(char::is_whitespace)
        .map(|(verb, value)| (verb.trim(), value.trim()))
        .unwrap_or((rest, ""));
    match verb {
        "manifest" | "plan" | "workers-plan" => Some(SlashCommand::ModelPoolManifest),
        "advice" | "capacity" | "expand" | "expansion" => Some(SlashCommand::ModelPoolAdvice),
        "smoke" | "check" | "routes" | "probe" => Some(SlashCommand::ModelPoolSmoke),
        "route" | "route-plan" => parse_pool_task_kind(value).map(SlashCommand::ModelPoolRoute),
        "watch" => parse_pool_watch_command(value),
        "call" | "ask" | "dispatch" => parse_pool_call_command(value),
        _ => None,
    }
}

fn parse_pool_watch_command(rest: &str) -> Option<SlashCommand> {
    let rest = rest.trim();
    if matches!(rest, "off" | "stop" | "clear" | "none") {
        return Some(SlashCommand::ModelPoolWatch(ModelPoolWatchCommand {
            interval_secs: DEFAULT_MODEL_POOL_WATCH_INTERVAL_SECS,
            max_iterations: None,
            enabled: false,
        }));
    }
    if rest.is_empty() || matches!(rest, "on" | "start") {
        return Some(SlashCommand::ModelPoolWatch(ModelPoolWatchCommand {
            interval_secs: DEFAULT_MODEL_POOL_WATCH_INTERVAL_SECS,
            max_iterations: None,
            enabled: true,
        }));
    }

    let mut parts = rest.split_whitespace();
    let interval_secs = parts
        .next()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)?;
    let max_iterations = match parts.next() {
        Some(value) => Some(value.parse::<usize>().ok().filter(|value| *value > 0)?),
        None => None,
    };
    if parts.next().is_some() {
        return None;
    }

    Some(SlashCommand::ModelPoolWatch(ModelPoolWatchCommand {
        interval_secs,
        max_iterations,
        enabled: true,
    }))
}

fn parse_pool_call_command(rest: &str) -> Option<SlashCommand> {
    let (kind, prompt) = rest
        .trim()
        .split_once(char::is_whitespace)
        .map(|(kind, prompt)| (kind.trim(), prompt.trim()))?;
    if prompt.is_empty() {
        return None;
    }
    parse_pool_task_kind(kind).map(|task_kind| SlashCommand::ModelPoolCall {
        task_kind,
        prompt: prompt.to_owned(),
    })
}

fn parse_pool_task_kind(rest: &str) -> Option<String> {
    let value = rest.trim().to_ascii_lowercase();
    match value.as_str() {
        "auto" | "summary" | "review" | "index" | "quality" => Some(value),
        "spare" => Some("index".to_owned()),
        "repo-index" | "repository-index" => Some("index".to_owned()),
        "test-gate" | "test" | "gate" => Some("test-gate".to_owned()),
        _ => None,
    }
}

fn parse_retrieval_command(rest: &str) -> Option<SlashCommand> {
    let rest = rest.trim();
    if rest.is_empty() {
        return None;
    }
    let (first, tail) = rest
        .split_once(char::is_whitespace)
        .map(|(first, tail)| (first.trim(), tail.trim()))
        .unwrap_or((rest, ""));
    if let Ok(limit) = first.parse::<usize>() {
        if tail.is_empty() {
            return None;
        }
        return Some(SlashCommand::RetrievalPreview {
            prompt: tail.to_owned(),
            limit: limit.max(1),
        });
    }
    Some(SlashCommand::RetrievalPreview {
        prompt: rest.to_owned(),
        limit: 5,
    })
}

fn parse_notes_command(rest: &str) -> Option<SlashCommand> {
    if rest.trim().is_empty() {
        return Some(SlashCommand::NotesShow);
    }
    let (verb, value) = rest
        .split_once(char::is_whitespace)
        .map(|(verb, value)| (verb.trim(), value.trim()))
        .unwrap_or((rest.trim(), ""));
    match verb {
        "show" | "list" => Some(SlashCommand::NotesShow),
        "add" | "append" if !value.is_empty() => Some(SlashCommand::NotesAdd(value.to_owned())),
        "set" | "replace" if !value.is_empty() => Some(SlashCommand::NotesSet(value.to_owned())),
        "clear" | "off" | "none" => Some(SlashCommand::NotesClear),
        "index" | "model-pool-index" => parse_index_notes_command(value),
        _ => None,
    }
}

fn parse_index_notes_command(rest: &str) -> Option<SlashCommand> {
    match rest.trim() {
        "" | "show" | "list" => Some(SlashCommand::IndexNotesShow),
        "clear" | "off" | "none" => Some(SlashCommand::IndexNotesClear),
        _ => None,
    }
}

fn parse_rust_check_command(rest: &str) -> Option<SlashCommand> {
    let (verb, value) = rest
        .split_once(char::is_whitespace)
        .map(|(verb, value)| (verb.trim(), value.trim()))
        .unwrap_or((rest.trim(), ""));
    match verb {
        "inline" | "code" if !value.is_empty() => {
            Some(SlashCommand::RustCheckInline(value.to_owned()))
        }
        "file" if !value.is_empty() => Some(SlashCommand::RustCheckFile(value.to_owned())),
        "edition" if !value.is_empty() => Some(SlashCommand::RustCheckEdition(value.to_owned())),
        "case" if value.is_empty() || value == "off" || value == "none" => {
            Some(SlashCommand::RustCheckCase(None))
        }
        "case" => Some(SlashCommand::RustCheckCase(Some(value.to_owned()))),
        "off" | "clear" | "none" => Some(SlashCommand::RustCheckClear),
        _ => None,
    }
}

fn parse_hygiene_command(rest: &str) -> Option<SlashCommand> {
    if rest.trim().is_empty() || matches!(rest.trim(), "show" | "check" | "report") {
        return Some(SlashCommand::Hygiene);
    }
    let (verb, value) = rest
        .split_once(char::is_whitespace)
        .map(|(verb, value)| (verb.trim(), value.trim()))
        .unwrap_or((rest.trim(), ""));
    match verb {
        "dry-run" | "quarantine" | "quarantine-dry-run" => {
            let limit = if value.is_empty() {
                20
            } else {
                value.parse::<usize>().ok()?.max(1)
            };
            Some(SlashCommand::HygieneQuarantineDryRun(limit))
        }
        _ => None,
    }
}

fn parse_repair_command(rest: &str) -> Option<SlashCommand> {
    if rest.trim().is_empty() || matches!(rest.trim(), "dry-run" | "preview" | "plan") {
        return Some(SlashCommand::ExperienceRepairDryRun(20));
    }
    let (verb, value) = rest
        .split_once(char::is_whitespace)
        .map(|(verb, value)| (verb.trim(), value.trim()))
        .unwrap_or((rest.trim(), ""));
    match verb {
        "dry-run" | "preview" | "plan" => {
            let limit = if value.is_empty() {
                20
            } else {
                value.parse::<usize>().ok()?.max(1)
            };
            Some(SlashCommand::ExperienceRepairDryRun(limit))
        }
        _ => None,
    }
}

fn parse_audit_command(rest: &str) -> Option<SlashCommand> {
    let rest = rest.trim();
    if rest.is_empty() || matches!(rest, "show" | "check" | "report") {
        return Some(SlashCommand::ExperienceCleanupAudit(20));
    }
    let (verb, value) = rest
        .split_once(char::is_whitespace)
        .map(|(verb, value)| (verb.trim(), value.trim()))
        .unwrap_or((rest, ""));
    if let Ok(limit) = verb.parse::<usize>() {
        return Some(SlashCommand::ExperienceCleanupAudit(limit.max(1)));
    }
    match verb {
        "show" | "check" | "report" => {
            let limit = if value.is_empty() {
                20
            } else {
                value.parse::<usize>().ok()?.max(1)
            };
            Some(SlashCommand::ExperienceCleanupAudit(limit))
        }
        _ => None,
    }
}

fn parse_sessions_command(value: &str) -> Option<(SessionFilter, usize)> {
    const DEFAULT_LIMIT: usize = 50;
    const MAX_LIMIT: usize = 500;

    let value = value.trim();
    if value.is_empty() {
        return Some((SessionFilter::All, DEFAULT_LIMIT));
    }
    let mut parts = value.split_whitespace();
    let first = parts.next()?;
    if let Ok(limit) = first.parse::<usize>() {
        return Some((SessionFilter::All, limit.clamp(1, MAX_LIMIT)));
    }
    let filter = SessionFilter::parse(first)?;
    let limit = parts
        .next()
        .and_then(|value| value.parse::<usize>().ok())
        .map(|limit| limit.clamp(1, MAX_LIMIT))
        .unwrap_or(DEFAULT_LIMIT);
    Some((filter, limit))
}

fn parse_endpoint(value: &str) -> Option<StreamEndpoint> {
    match value.trim() {
        "chat" => Some(StreamEndpoint::Chat),
        "generate" => Some(StreamEndpoint::Generate),
        "business-cycle" | "business" | "cycle" => Some(StreamEndpoint::BusinessCycle),
        _ => None,
    }
}

fn parse_output(value: &str) -> Option<String> {
    match value.trim() {
        "raw" | "enhanced" => Some(value.trim().to_owned()),
        _ => None,
    }
}

fn parse_profile(value: &str) -> Option<String> {
    match value.trim() {
        "coding" | "general" | "writing" | "long" => Some(value.trim().to_owned()),
        _ => None,
    }
}

fn parse_feedback(value: &str) -> Option<String> {
    let parsed = value.trim().parse::<f32>().ok()?;
    (0.0..=1.0)
        .contains(&parsed)
        .then(|| format!("{parsed:.3}"))
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim() {
        "on" | "true" | "yes" | "1" => Some(true),
        "off" | "false" | "no" | "0" => Some(false),
        _ => None,
    }
}

fn parse_optional_max_tokens(value: &str) -> Option<Option<usize>> {
    let value = value.trim().to_ascii_lowercase();
    if matches!(
        value.as_str(),
        "auto" | "off" | "default" | "backend" | "none"
    ) {
        return Some(None);
    }
    parse_positive_usize(&value).map(|value| Some(value.min(262_144)))
}

fn parse_positive_usize(value: &str) -> Option<usize> {
    value
        .trim()
        .parse::<usize>()
        .ok()
        .filter(|value| *value > 0)
}
