use std::io::{self, Write};
use std::time::Duration;

use crate::backend::{
    BackendHealth, backend_prompt_block_reason, call_backend_event_stream, call_backend_health,
    call_backend_model_pool_status,
};
use crate::config::Config;
use crate::json::{json_array_field, json_bool_field, json_number_field, json_string_field};
use crate::model_pool_advice::model_pool_advice_json;
use crate::request::{ChatMessage, ChatRequest, LabEndpoint, request_context_preview};

const DEFAULT_CONTEXT_MESSAGES: usize = 64;
const MAX_CONTEXT_MESSAGES: usize = 256;
const DEFAULT_MAX_TOKENS: usize = 262_144;

#[derive(Debug, Clone)]
struct ReplState {
    endpoint: LabEndpoint,
    output: String,
    profile: String,
    feedback_amount: String,
    max_tokens: usize,
    self_improve: bool,
    rust_check_code: Option<String>,
    backend_response_timeout: Duration,
    max_context_messages: usize,
    messages: Vec<ChatMessage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ReplCommand {
    Help,
    Quit,
    Clear,
    Context,
    Status,
    PoolAdvice,
    Show,
    Endpoint(LabEndpoint),
    Output(String),
    Profile(String),
    Feedback(String),
    MaxTokens(usize),
    ContextWindow(usize),
    SelfImprove(bool),
    RustCheck(String),
    RustClear,
}

pub(crate) fn run(config: Config) -> io::Result<()> {
    let mut state = ReplState::default();
    state.backend_response_timeout = config.backend_response_timeout;
    state.max_context_messages = config.context_messages;
    println!("rust-norion interactive lab");
    println!("backend: {}", config.backend);
    println!(
        "backend response timeout: {}s",
        config.backend_response_timeout.as_secs()
    );
    println!("type /help for commands, /quit to exit");
    println!();

    let stdin = io::stdin();
    loop {
        print!("norion:{}> ", state.endpoint.as_label());
        io::stdout().flush()?;

        let mut line = String::new();
        if stdin.read_line(&mut line)? == 0 {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(command) = parse_command(line) {
            if apply_command(command, &mut state, &config.backend)? {
                break;
            }
            continue;
        }

        send_prompt(&config.backend, &mut state, line)?;
    }

    Ok(())
}

impl Default for ReplState {
    fn default() -> Self {
        Self {
            endpoint: LabEndpoint::Chat,
            output: "raw".to_owned(),
            profile: "coding".to_owned(),
            feedback_amount: "0.5".to_owned(),
            max_tokens: DEFAULT_MAX_TOKENS,
            self_improve: true,
            rust_check_code: None,
            backend_response_timeout: crate::config::DEFAULT_BACKEND_RESPONSE_TIMEOUT,
            max_context_messages: DEFAULT_CONTEXT_MESSAGES,
            messages: Vec::new(),
        }
    }
}

fn parse_command(line: &str) -> Option<ReplCommand> {
    let command = line.strip_prefix('/')?;
    let (name, rest) = command
        .split_once(char::is_whitespace)
        .map(|(name, rest)| (name, rest.trim()))
        .unwrap_or((command, ""));
    match name {
        "help" | "?" => Some(ReplCommand::Help),
        "quit" | "exit" => Some(ReplCommand::Quit),
        "clear" => Some(ReplCommand::Clear),
        "context" | "ctx" => Some(ReplCommand::Context),
        "status" => Some(ReplCommand::Status),
        "pool-advice" | "model-pool-advice" | "pool" => Some(ReplCommand::PoolAdvice),
        "show" => Some(ReplCommand::Show),
        "endpoint" | "mode" => parse_endpoint(rest).map(ReplCommand::Endpoint),
        "output" => parse_output(rest).map(ReplCommand::Output),
        "profile" => parse_profile(rest).map(ReplCommand::Profile),
        "feedback" => parse_feedback(rest).map(ReplCommand::Feedback),
        "max" | "max-tokens" => parse_max_tokens(rest).map(ReplCommand::MaxTokens),
        "context-window" | "context-messages" | "ctx-window" => {
            parse_context_window(rest).map(ReplCommand::ContextWindow)
        }
        "self-improve" | "self" => parse_bool(rest).map(ReplCommand::SelfImprove),
        "rust" | "rust-check" => Some(ReplCommand::RustCheck(rest.to_owned())),
        "rust-clear" => Some(ReplCommand::RustClear),
        _ => {
            println!("unknown command: /{name}. type /help");
            Some(ReplCommand::Show)
        }
    }
}

fn apply_command(command: ReplCommand, state: &mut ReplState, backend: &str) -> io::Result<bool> {
    match command {
        ReplCommand::Help => print_help(),
        ReplCommand::Quit => return Ok(true),
        ReplCommand::Clear => {
            state.messages.clear();
            println!("conversation cleared");
        }
        ReplCommand::Context => print_context(state),
        ReplCommand::Status => print_status(backend, state.backend_response_timeout),
        ReplCommand::PoolAdvice => print_model_pool_advice(backend, state.backend_response_timeout),
        ReplCommand::Show => print_settings(state),
        ReplCommand::Endpoint(endpoint) => {
            state.endpoint = endpoint;
            println!("endpoint: {}", endpoint.as_label());
        }
        ReplCommand::Output(output) => {
            state.output = output;
            println!("output: {}", state.output);
        }
        ReplCommand::Profile(profile) => {
            state.profile = profile;
            println!("profile: {}", state.profile);
        }
        ReplCommand::Feedback(amount) => {
            state.feedback_amount = amount;
            println!("feedback_amount: {}", state.feedback_amount);
        }
        ReplCommand::MaxTokens(max_tokens) => {
            state.max_tokens = max_tokens;
            println!("max_tokens: {}", state.max_tokens);
        }
        ReplCommand::ContextWindow(max_messages) => {
            state.max_context_messages = max_messages;
            trim_history(&mut state.messages, state.max_context_messages);
            println!("context_messages: {}", state.max_context_messages);
        }
        ReplCommand::SelfImprove(enabled) => {
            state.self_improve = enabled;
            println!("self_improve: {}", state.self_improve);
        }
        ReplCommand::RustCheck(code) => {
            if code.trim().is_empty() {
                println!("usage: /rust pub fn ok() -> bool {{ true }}");
            } else {
                state.rust_check_code = Some(code);
                println!("rust_check_code set");
            }
        }
        ReplCommand::RustClear => {
            state.rust_check_code = None;
            println!("rust_check_code cleared");
        }
    }
    Ok(false)
}

fn send_prompt(backend: &str, state: &mut ReplState, prompt: &str) -> io::Result<()> {
    match call_backend_health(backend, state.backend_response_timeout) {
        Ok(health) => {
            if let Some(reason) = backend_prompt_block_reason(&health) {
                println!("[blocked] {reason}");
                return Ok(());
            }
        }
        Err(error) => {
            println!("[blocked] backend health check failed: {error}");
            return Ok(());
        }
    }

    let outgoing_messages = outgoing_messages(state, prompt);
    let request = ChatRequest {
        prompt: prompt.to_owned(),
        messages: outgoing_messages.clone(),
        profile: state.profile.clone(),
        output: state.output.clone(),
        endpoint: state.endpoint,
        max_tokens: state.max_tokens,
        feedback_amount: state.feedback_amount.clone(),
        rust_check_code: state.rust_check_code.clone(),
        self_improve: state.self_improve,
    };
    let mut streamed_answer = String::new();
    let mut final_answer = None;
    let mut stream_error_event = None;

    println!("\n[request]\n{}", request_context_preview(&request));
    let result = call_backend_event_stream(
        backend,
        &request,
        state.backend_response_timeout,
        &mut |event, data| {
            match event {
                "delta" => {
                    print!("{data}");
                    let _ = io::stdout().flush();
                    streamed_answer.push_str(data);
                }
                "stage" => println!("\n[stage] {data}"),
                "status" | "heartbeat" => println!("\n[{event}] {data}"),
                "meta" => println!("\n[meta] {data}"),
                "final" => {
                    final_answer = final_answer_from_json(data);
                    print_final_summary(data);
                }
                "done" => println!("\n[DONE]"),
                "error" => {
                    stream_error_event = Some(data.to_owned());
                    println!("\n[error] {data}");
                }
                _ => println!("\n[{event}] {data}"),
            }
            Ok(())
        },
    );

    if let Err(error) = result {
        println!("\n[error] {error}");
        return Ok(());
    }
    if let Some(error) = stream_error_event {
        println!("[context] assistant partial output discarded after stream error: {error}");
        return Ok(());
    }

    let assistant_answer =
        assistant_answer_for_history(final_answer, streamed_answer).unwrap_or_default();
    if state.endpoint == LabEndpoint::Chat && !assistant_answer.trim().is_empty() {
        state.messages = outgoing_messages;
        state.messages.push(ChatMessage {
            role: "assistant".to_owned(),
            content: assistant_answer,
        });
        trim_history(&mut state.messages, state.max_context_messages);
    }
    Ok(())
}

fn outgoing_messages(state: &ReplState, prompt: &str) -> Vec<ChatMessage> {
    let mut messages = state
        .messages
        .iter()
        .rev()
        .take(state.max_context_messages.saturating_sub(1))
        .cloned()
        .collect::<Vec<_>>();
    messages.reverse();
    messages.push(ChatMessage {
        role: "user".to_owned(),
        content: prompt.to_owned(),
    });
    messages
}

fn assistant_answer_for_history(
    final_answer: Option<String>,
    streamed_answer: String,
) -> Option<String> {
    final_answer.or_else(|| (!streamed_answer.trim().is_empty()).then_some(streamed_answer))
}

fn trim_history(messages: &mut Vec<ChatMessage>, max_context_messages: usize) {
    if messages.len() > max_context_messages {
        let drop_count = messages.len() - max_context_messages;
        messages.drain(..drop_count);
    }
}

fn print_status(backend: &str, response_timeout: Duration) {
    match call_backend_health(backend, response_timeout) {
        Ok(health) => {
            for line in status_lines(&health) {
                println!("{line}");
            }
        }
        Err(error) => println!("status error: {error}"),
    }
}

fn status_lines(health: &BackendHealth) -> Vec<String> {
    let mut lines = vec![
        format!("backend ok: {}", health.ok),
        format!(
            "runtime_mode: {}",
            option_text(health.runtime_mode.as_deref())
        ),
        format!(
            "gemma_runtime_reachable: {}",
            bool_text(health.gemma_runtime_reachable)
        ),
        format!(
            "active_engine_requests: {}",
            option_text(health.active_engine_requests.as_deref())
        ),
        format!("readiness_ok: {}", bool_text(health.readiness_ok)),
        format!("safe_device_ok: {}", bool_text(health.safe_device_ok)),
    ];
    if let Some(reason) = backend_prompt_block_reason(health) {
        lines.push(format!("prompt_gate: blocked: {reason}"));
    } else {
        lines.push("prompt_gate: ready".to_owned());
    }
    if let Some(hygiene) = &health.experience_hygiene {
        lines.push(format!(
            "experience_file: {}",
            option_text(hygiene.experience_file.as_deref())
        ));
        lines.push(format!(
            "experience_hygiene: checked={} clean={} findings={} quarantine_candidates={} repairable_legacy_metadata_lessons={} repairable_index_records={}",
            bool_text(hygiene.checked),
            bool_text(hygiene.clean),
            option_text(hygiene.findings.as_deref()),
            option_text(hygiene.quarantine_candidates.as_deref()),
            option_text(hygiene.repairable_legacy_metadata_lessons.as_deref()),
            option_text(hygiene.repairable_index_records.as_deref())
        ));
        if let Some(index) = &hygiene.index {
            lines.push(format!(
                "experience_index: retrieval_ready={} risk_level={} quality_score={} noisy_records={} duplicate_outputs={}",
                bool_text(index.retrieval_ready),
                option_text(index.risk_level.as_deref()),
                option_text(index.quality_score.as_deref()),
                option_text(index.noisy_records.as_deref()),
                option_text(index.duplicate_outputs.as_deref())
            ));
        }
    }
    lines
}

fn print_model_pool_advice(backend: &str, response_timeout: Duration) {
    match call_backend_model_pool_status(backend, response_timeout) {
        Ok(status_body) => {
            let advice = model_pool_advice_json(&status_body);
            println!("model_pool_advice:");
            println!(
                "  advice: {}",
                option_text(json_string_field(&advice, "advice").as_deref())
            );
            println!(
                "  safe_to_enable_pool_workers: {}",
                json_bool_field(&advice, "safe_to_enable_pool_workers")
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "unknown".to_owned())
            );
            println!(
                "  extra_quality_12b_detected: {}",
                json_bool_field(&advice, "extra_quality_12b_detected")
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "unknown".to_owned())
            );
            println!(
                "  worker_shape: quality={} helpers={}",
                option_text(json_number_field(&advice, "quality_worker_count").as_deref()),
                option_text(json_number_field(&advice, "helper_worker_count").as_deref())
            );
            println!(
                "  expected_helper_roles: {}",
                json_string_array_text(&advice, "expected_helper_roles")
            );
            println!(
                "  missing_helper_roles: {}",
                json_string_array_text(&advice, "missing_helper_roles")
            );
            println!(
                "  recommended_launch_order: {}",
                json_string_array_text(&advice, "recommended_launch_order")
            );
            println!(
                "  next_step: {}",
                option_text(json_string_field(&advice, "next_step").as_deref())
            );
            println!(
                "  reason: {}",
                option_text(json_string_field(&advice, "reason").as_deref())
            );
            println!("  raw: {advice}");
        }
        Err(error) => println!("model_pool_advice error: {error}"),
    }
}

fn print_help() {
    println!("commands:");
    println!("  /status                  show backend/Gemma health");
    println!("  /pool-advice             show read-only Apple model-pool expansion advice");
    println!("  /mode chat|generate|business-cycle");
    println!("  /output raw|enhanced");
    println!("  /profile coding|general|writing|long");
    println!("  /feedback 0.0..1.0");
    println!("  /max 1..262144          set generation max_tokens");
    println!("{}", context_window_help_line());
    println!("  /self on|off");
    println!("  /rust <code>             set Rust check code for business-cycle");
    println!("  /rust-clear              clear Rust check code");
    println!("  /clear                   clear chat history");
    println!("  /context                 show current chat history context");
    println!("  /show                    show current settings");
    println!("  /quit                    exit");
}

fn context_window_help_line() -> &'static str {
    "  /context-window 2..256  set chat history message count, not a token limit"
}

fn print_settings(state: &ReplState) {
    println!("endpoint: {}", state.endpoint.as_label());
    println!("output: {}", state.output);
    println!("profile: {}", state.profile);
    println!("feedback_amount: {}", state.feedback_amount);
    println!("max_tokens: {}", state.max_tokens);
    println!("context_messages: {}", state.max_context_messages);
    println!("self_improve: {}", state.self_improve);
    println!("rust_check_code: {}", state.rust_check_code.is_some());
    println!("history_messages: {}", state.messages.len());
}

fn print_context(state: &ReplState) {
    println!(
        "context messages: {}/{}",
        state.messages.len(),
        state.max_context_messages
    );
    if state.messages.is_empty() {
        println!("  empty");
        return;
    }
    for (index, message) in state.messages.iter().enumerate() {
        println!(
            "  {}. {}: {}",
            index + 1,
            message.role,
            preview_text(&message.content, 160)
        );
    }
}

fn preview_text(text: &str, max_chars: usize) -> String {
    let normalized = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" / ");
    let text = if normalized.is_empty() {
        text.trim().to_owned()
    } else {
        normalized
    };
    if text.chars().count() <= max_chars {
        return text;
    }
    let keep_chars = max_chars.saturating_sub(3);
    let mut preview = text.chars().take(keep_chars).collect::<String>();
    preview.push_str("...");
    preview
}

fn print_final_summary(data: &str) {
    let answer = final_answer_from_json(data);
    let elapsed = json_number_field(data, "elapsed_ms").unwrap_or_else(|| "?".to_owned());
    let tokens = json_number_field(data, "runtime_token_count").unwrap_or_else(|| "?".to_owned());
    if final_payload_is_business_cycle(data) {
        let passed = json_bool_field(data, "passed")
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".to_owned());
        let feedback =
            json_number_field(data, "feedback_applied").unwrap_or_else(|| "?".to_owned());
        println!(
            "\n[final] business_cycle passed={passed} elapsed_ms={elapsed} runtime_tokens={tokens}"
        );
        println!("[final] feedback_applied={feedback}");
    } else {
        println!("\n[final] elapsed_ms={elapsed} runtime_tokens={tokens}");
    }
    if let Some(answer) = answer.filter(|answer| !answer.trim().is_empty()) {
        println!("\n[final answer]\n{answer}");
    }
}

fn final_answer_from_json(data: &str) -> Option<String> {
    json_string_field(data, "answer")
}

fn final_payload_is_business_cycle(data: &str) -> bool {
    json_array_field(data, "business_cycle").is_some()
        || crate::json::json_object_field(data, "business_cycle").is_some()
        || json_string_field(data, "gate").is_some_and(|gate| gate == "business_cycle")
        || json_string_field(data, "endpoint").is_some_and(|endpoint| {
            matches!(endpoint.as_str(), "business-cycle" | "business_cycle")
        })
}

fn parse_endpoint(value: &str) -> Option<LabEndpoint> {
    match value.trim() {
        "chat" => Some(LabEndpoint::Chat),
        "generate" => Some(LabEndpoint::Generate),
        "business-cycle" | "business" | "cycle" => Some(LabEndpoint::BusinessCycle),
        _ => {
            println!("endpoint must be chat|generate|business-cycle");
            None
        }
    }
}

fn parse_output(value: &str) -> Option<String> {
    match value.trim() {
        "raw" | "enhanced" => Some(value.trim().to_owned()),
        _ => {
            println!("output must be raw|enhanced");
            None
        }
    }
}

fn parse_profile(value: &str) -> Option<String> {
    match value.trim() {
        "coding" | "general" | "writing" | "long" => Some(value.trim().to_owned()),
        _ => {
            println!("profile must be coding|general|writing|long");
            None
        }
    }
}

fn parse_feedback(value: &str) -> Option<String> {
    let parsed = value.trim().parse::<f32>().ok()?;
    if (0.0..=1.0).contains(&parsed) {
        Some(format!("{parsed:.3}"))
    } else {
        println!("feedback must be between 0.0 and 1.0");
        None
    }
}

fn parse_max_tokens(value: &str) -> Option<usize> {
    match value.trim().parse::<usize>() {
        Ok(value) => Some(value.clamp(1, 262_144)),
        Err(_) => {
            println!("max_tokens must be a number");
            None
        }
    }
}

fn parse_context_window(value: &str) -> Option<usize> {
    match value.trim().parse::<usize>() {
        Ok(value) => Some(value.clamp(2, MAX_CONTEXT_MESSAGES)),
        Err(_) => {
            println!("context window must be a number");
            None
        }
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim() {
        "on" | "true" | "yes" | "1" => Some(true),
        "off" | "false" | "no" | "0" => Some(false),
        _ => {
            println!("value must be on|off");
            None
        }
    }
}

fn option_text(value: Option<&str>) -> &str {
    value.unwrap_or("unknown")
}

fn bool_text(value: Option<bool>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_owned())
}

fn json_string_array_text(body: &str, field: &str) -> String {
    let Some(array) = json_array_field(body, field) else {
        return "unknown".to_owned();
    };
    let mut values = Vec::new();
    let mut input = array.trim_start();
    while !input.is_empty() {
        let wrapped = format!("{{\"value\":{input}}}");
        let Some(value) = json_string_field(&wrapped, "value") else {
            break;
        };
        values.push(value);
        let Some(after_string) = input.get(input.find('"').unwrap_or(0) + 1..) else {
            break;
        };
        let mut escaped = false;
        let mut end = None;
        for (index, character) in after_string.char_indices() {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                end = Some(index + 2);
                break;
            }
        }
        let Some(end) = end else {
            break;
        };
        input = input
            .get(end..)
            .unwrap_or_default()
            .trim_start()
            .strip_prefix(',')
            .unwrap_or_default()
            .trim_start();
    }
    if values.is_empty() {
        "none".to_owned()
    } else {
        values.join(",")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::{
        BackendExperienceHygiene, BackendExperienceIndex, BackendHealth, BackendLastInference,
    };

    #[test]
    fn parses_mode_command() {
        assert_eq!(
            parse_command("/mode business-cycle"),
            Some(ReplCommand::Endpoint(LabEndpoint::BusinessCycle))
        );
    }

    #[test]
    fn parses_context_command() {
        assert_eq!(parse_command("/context"), Some(ReplCommand::Context));
        assert_eq!(parse_command("/ctx"), Some(ReplCommand::Context));
    }

    #[test]
    fn parses_context_window_command() {
        assert_eq!(
            parse_command("/context-window 128"),
            Some(ReplCommand::ContextWindow(128))
        );
        assert_eq!(
            parse_command("/ctx-window 999"),
            Some(ReplCommand::ContextWindow(256))
        );
    }

    #[test]
    fn help_line_distinguishes_context_messages_from_tokens() {
        assert!(context_window_help_line().contains("message count"));
        assert!(context_window_help_line().contains("not a token limit"));
    }

    #[test]
    fn parses_pool_advice_command() {
        assert_eq!(parse_command("/pool-advice"), Some(ReplCommand::PoolAdvice));
        assert_eq!(
            parse_command("/model-pool-advice"),
            Some(ReplCommand::PoolAdvice)
        );
        assert_eq!(parse_command("/pool"), Some(ReplCommand::PoolAdvice));
    }

    #[test]
    fn formats_json_string_arrays_for_pool_advice() {
        assert_eq!(
            json_string_array_text(
                "{\"missing_helper_roles\":[\"summary\",\"review\",\"test-gate\"]}",
                "missing_helper_roles"
            ),
            "summary,review,test-gate"
        );
        assert_eq!(
            json_string_array_text("{\"missing_helper_roles\":[]}", "missing_helper_roles"),
            "none"
        );
    }

    #[test]
    fn status_lines_include_experience_hygiene_and_index_debt() {
        let health = BackendHealth {
            ok: true,
            service: Some("rust-norion".to_owned()),
            requests_seen: Some("9".to_owned()),
            active_engine_requests: Some("0".to_owned()),
            engine_busy: Some(false),
            runtime_mode: Some("built-in".to_owned()),
            gemma_runtime_server: None,
            gemma_runtime_reachable: None,
            gemma_runtime_model: None,
            gemma_runtime_context_window: None,
            gemma_runtime_train_context_window: None,
            gemma_runtime_vocab_size: None,
            gemma_runtime_metadata_error: None,
            readiness_ok: Some(true),
            safe_device_ok: Some(true),
            readiness_failures: Vec::new(),
            safe_device_failures: Vec::new(),
            device_primary_lane: None,
            device_memory_mode: None,
            experience_hygiene: Some(BackendExperienceHygiene {
                experience_file: Some("D:\\rust-norion\\target\\state\\experience.ndkv".to_owned()),
                checked: Some(true),
                clean: Some(false),
                findings: Some("1".to_owned()),
                quarantine_candidates: Some("0".to_owned()),
                repairable_legacy_metadata_lessons: Some("0".to_owned()),
                repairable_index_records: Some("1".to_owned()),
                index: Some(BackendExperienceIndex {
                    total_records: Some("42".to_owned()),
                    noisy_records: Some("2".to_owned()),
                    duplicate_outputs: Some("1".to_owned()),
                    quality_score: Some("0.340000".to_owned()),
                    retrieval_ready: Some(false),
                    risk_level: Some("blocked".to_owned()),
                }),
            }),
            active_requests: Vec::new(),
            last_inference: None::<BackendLastInference>,
            error: None,
        };

        let lines = status_lines(&health);

        assert!(lines.iter().any(|line| {
            line == "prompt_gate: blocked: backend experience repair required: repairable_index_records=1; dry-run repair before chatting"
        }));
        assert!(lines.iter().any(|line| {
            line.contains("repairable_index_records=1") && line.contains("quarantine_candidates=0")
        }));
        assert!(lines.iter().any(|line| {
            line == "experience_index: retrieval_ready=false risk_level=blocked quality_score=0.340000 noisy_records=2 duplicate_outputs=1"
        }));
    }

    #[test]
    fn keeps_recent_chat_history() {
        let state = ReplState {
            messages: (0..70)
                .map(|index| ChatMessage {
                    role: "user".to_owned(),
                    content: format!("m{index}"),
                })
                .collect(),
            ..ReplState::default()
        };

        let messages = outgoing_messages(&state, "next");

        assert_eq!(messages.len(), DEFAULT_CONTEXT_MESSAGES);
        assert_eq!(messages.last().unwrap().content, "next");
        assert_eq!(messages.first().unwrap().content, "m7");
    }

    #[test]
    fn outgoing_messages_use_configured_context_window() {
        let state = ReplState {
            max_context_messages: 128,
            messages: (0..140)
                .map(|index| ChatMessage {
                    role: "user".to_owned(),
                    content: format!("m{index}"),
                })
                .collect(),
            ..ReplState::default()
        };

        let messages = outgoing_messages(&state, "next");

        assert_eq!(messages.len(), 128);
        assert_eq!(messages.last().unwrap().content, "next");
        assert_eq!(messages.first().unwrap().content, "m13");
    }

    #[test]
    fn extracts_final_answer_from_json() {
        let answer = final_answer_from_json("{\"answer\":\"你好\"}");

        assert_eq!(answer.as_deref(), Some("你好"));
    }

    #[test]
    fn final_payload_business_cycle_detection_uses_structured_fields() {
        assert!(final_payload_is_business_cycle(
            "{\"business_cycle\":{\"passed\":true},\"answer\":\"ok\"}"
        ));
        assert!(final_payload_is_business_cycle(
            "{\"gate\":\"business_cycle\",\"answer\":\"ok\"}"
        ));
        assert!(final_payload_is_business_cycle(
            "{\"endpoint\":\"business-cycle\",\"answer\":\"ok\"}"
        ));
    }

    #[test]
    fn final_payload_business_cycle_detection_ignores_answer_text() {
        assert!(!final_payload_is_business_cycle(
            r#"{"answer":"literal \"business_cycle\" text","elapsed_ms":1}"#
        ));
        assert!(!final_payload_is_business_cycle(
            r#"{"note":"\"gate\":\"business_cycle\"","answer":"chat answer"}"#
        ));
    }

    #[test]
    fn stream_error_events_do_not_update_history() {
        let mut state = ReplState {
            messages: vec![ChatMessage {
                role: "user".to_owned(),
                content: "before".to_owned(),
            }],
            ..ReplState::default()
        };
        let outgoing = outgoing_messages(&state, "next");
        let stream_error_event = Some("backend stream truncated".to_owned());
        let streamed_answer = "partial".to_owned();

        if stream_error_event.is_none() {
            if let Some(answer) = assistant_answer_for_history(None, streamed_answer) {
                state.messages = outgoing;
                state.messages.push(ChatMessage {
                    role: "assistant".to_owned(),
                    content: answer,
                });
            }
        }

        assert_eq!(state.messages.len(), 1);
        assert_eq!(state.messages[0].content, "before");
    }
}
