use crate::reflection::ReasoningStep;
use crate::runtime::{RuntimeToken, option_f32_display, option_usize_display};

use super::CommandTextOutputFilter;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(in crate::runtime) struct MistralRsCliStats {
    pub(in crate::runtime) prompt_tokens: Option<usize>,
    pub(in crate::runtime) decode_tokens: Option<usize>,
    pub(in crate::runtime) decode_tokens_per_second: Option<f32>,
}

pub(in crate::runtime) fn filter_command_text_output(
    stdout: &str,
    filter: CommandTextOutputFilter,
) -> String {
    match filter {
        CommandTextOutputFilter::None => stdout.trim().to_owned(),
        CommandTextOutputFilter::MistralRsCli => sanitize_mistralrs_cli_text(stdout),
    }
}

fn sanitize_mistralrs_cli_text(stdout: &str) -> String {
    let without_ansi = strip_ansi_escape_sequences(stdout);
    let content = if let Some(stats_start) = find_mistralrs_stats_start(&without_ansi) {
        &without_ansi[..stats_start]
    } else {
        &without_ansi
    };
    content.trim().to_owned()
}

pub(in crate::runtime) fn parse_mistralrs_cli_stats(stdout: &str) -> Option<MistralRsCliStats> {
    let without_ansi = strip_ansi_escape_sequences(stdout);
    let stats_start = find_mistralrs_stats_start(&without_ansi)?;
    let stats = without_ansi
        .get(stats_start..)?
        .trim_start_matches(['\r', '\n'])
        .strip_prefix("Stats:")?;
    let mut parsed = MistralRsCliStats::default();

    for line in stats.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("Prompt:") {
            parsed.prompt_tokens = parse_stats_token_count(rest);
        } else if let Some(rest) = line.strip_prefix("Decode:") {
            parsed.decode_tokens = parse_stats_token_count(rest);
            parsed.decode_tokens_per_second = parse_decode_tokens_per_second(rest);
        }
    }

    (parsed.prompt_tokens.is_some()
        || parsed.decode_tokens.is_some()
        || parsed.decode_tokens_per_second.is_some())
    .then_some(parsed)
}

fn find_mistralrs_stats_start(value: &str) -> Option<usize> {
    if value.starts_with("Stats:\n") || value.starts_with("Stats:\r\n") {
        return Some(0);
    }
    value
        .find("\nStats:\n")
        .or_else(|| value.find("\r\nStats:\r\n"))
        .map(|index| index + 1)
}

fn parse_stats_token_count(value: &str) -> Option<usize> {
    let trimmed = value.trim_start();
    let digits = trimmed
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        None
    } else {
        digits.parse::<usize>().ok()
    }
}

fn parse_decode_tokens_per_second(value: &str) -> Option<f32> {
    let (_, after_comma) = value.split_once(',')?;
    let rate = after_comma
        .trim()
        .trim_end_matches("T/s")
        .trim()
        .parse::<f32>()
        .ok()?;
    rate.is_finite().then_some(rate.max(0.0))
}

pub(in crate::runtime::command) fn mistralrs_cli_reported_tokens(
    stats: MistralRsCliStats,
) -> Vec<RuntimeToken> {
    const MAX_REPORTED_TOKENS: usize = 100_000;
    let count = stats.decode_tokens.unwrap_or(0).min(MAX_REPORTED_TOKENS);
    (0..count)
        .map(|_| RuntimeToken::new("mistralrs_decode"))
        .collect()
}

pub(in crate::runtime::command) fn mistralrs_cli_stats_trace(
    stats: MistralRsCliStats,
) -> ReasoningStep {
    ReasoningStep::new(
        "mistralrs_cli_stats",
        format!(
            "reported prompt_tokens={} decode_tokens={} decode_tps={}",
            option_usize_display(stats.prompt_tokens),
            option_usize_display(stats.decode_tokens),
            option_f32_display(stats.decode_tokens_per_second)
        ),
        0.72,
    )
}

fn strip_ansi_escape_sequences(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut stripped = String::with_capacity(input.len());
    let mut index = 0;
    while index < input.len() {
        if bytes[index] == 0x1b && index + 1 < input.len() && bytes[index + 1] == b'[' {
            index += 2;
            while index < input.len() {
                let byte = bytes[index];
                index += 1;
                if (0x40..=0x7e).contains(&byte) {
                    break;
                }
            }
            continue;
        }

        let Some(ch) = input[index..].chars().next() else {
            break;
        };
        stripped.push(ch);
        index += ch.len_utf8();
    }
    stripped
}
