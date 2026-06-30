use super::model::ExperienceMatch;
use super::noise::{
    strip_reflection_lesson_suffix, strip_response_lesson_prefix, strip_reusable_text_prefixes,
    text_has_metadata_lesson_shape, text_has_transcript_shape,
};
use super::text_normalize::normalized_marker_spans;

const HINT_TEXT_CHARS: usize = 280;

pub fn render_experience_hint(experience: &ExperienceMatch) -> String {
    let gist_summaries = clean_gist_summaries(&experience.gist_hints, 2);
    let usable_text = if text_has_metadata_lesson_shape(&experience.lesson) {
        gist_summaries
            .first()
            .cloned()
            .unwrap_or_else(|| "prior accepted result has no reusable lesson text".to_owned())
    } else {
        reusable_text(&experience.lesson)
            .unwrap_or_else(|| compact_hint_text(&experience.lesson, HINT_TEXT_CHARS))
    };
    let gist_text = if gist_summaries.is_empty() {
        "none".to_owned()
    } else {
        gist_summaries.join(" | ")
    };

    format!(
        "{} score={:.3} quality={:.3} reward={:.3}/{}{}{} gist_summaries={}",
        usable_text,
        experience.score,
        experience.quality,
        experience.process_reward,
        experience.reward_action.as_str(),
        route_budget_hint(experience),
        reflection_hint(experience),
        gist_text
    )
}

fn route_budget_hint(experience: &ExperienceMatch) -> String {
    if experience.used_memory_count == 0
        && experience.route_attention_tokens == 0
        && experience.route_fast_tokens == 0
    {
        return String::new();
    }

    format!(
        " used_memories={} route_threshold={:.3} route_attention_fraction={:.3} route_attention_tokens={} route_fast_tokens={}",
        experience.used_memory_count,
        experience.route_threshold,
        experience.route_attention_fraction,
        experience.route_attention_tokens,
        experience.route_fast_tokens
    )
}

fn reflection_hint(experience: &ExperienceMatch) -> String {
    let issues = compact_reflection_items(&experience.reflection_issue_codes, 3, 80);
    let actions = compact_reflection_items(&experience.revision_actions, 2, 120);
    if issues.is_empty() && actions.is_empty() {
        return String::new();
    }

    let mut parts = Vec::with_capacity(2);
    if !issues.is_empty() {
        parts.push(format!("reflection_issues={}", issues.join("|")));
    }
    if !actions.is_empty() {
        parts.push(format!("revision_actions={}", actions.join("|")));
    }
    format!(" {}", parts.join(" "))
}

fn compact_reflection_items(items: &[String], limit: usize, max_chars: usize) -> Vec<String> {
    items
        .iter()
        .filter_map(|item| {
            let value = compact_hint_text(item, max_chars);
            (!value.is_empty()).then_some(value)
        })
        .take(limit)
        .collect()
}

fn clean_gist_summaries(gist_hints: &[String], limit: usize) -> Vec<String> {
    gist_hints
        .iter()
        .filter_map(|hint| {
            let summary = gist_hint_summary(hint).unwrap_or(hint);
            reusable_text(summary)
        })
        .take(limit.max(1))
        .collect()
}

fn gist_hint_summary(hint: &str) -> Option<&str> {
    normalized_marker_spans(hint, "summary=")
        .into_iter()
        .filter(|(index, _)| summary_marker_has_field_boundary(hint, *index))
        .filter_map(|(index, end)| hint.get(end..).map(|summary| (index, summary)))
        .max_by_key(|(index, _)| *index)
        .map(|(_, summary)| summary)
}

fn summary_marker_has_field_boundary(value: &str, index: usize) -> bool {
    index == 0
        || value
            .get(..index)
            .and_then(|prefix| prefix.chars().next_back())
            .is_some_and(char::is_whitespace)
}

fn reusable_text(value: &str) -> Option<String> {
    let trimmed = clean_reusable_text_body(value);
    if trimmed.is_empty()
        || text_has_metadata_lesson_shape(trimmed)
        || text_has_transcript_shape(trimmed)
    {
        return None;
    }

    Some(compact_hint_text(trimmed, HINT_TEXT_CHARS))
}

fn clean_reusable_text_body(value: &str) -> &str {
    let mut current = value.trim();
    for _ in 0..4 {
        let before_len = current.len();
        current = strip_response_lesson_prefix(current).unwrap_or(current);
        current = strip_reflection_lesson_suffix(current);
        current = strip_reusable_text_prefixes(current);
        if current.len() == before_len {
            break;
        }
    }
    current
}

fn compact_hint_text(value: &str, max_chars: usize) -> String {
    let mut out = String::new();
    let mut previous_space = false;
    for ch in value.chars().take(max_chars) {
        if ch.is_whitespace() {
            if !previous_space {
                out.push(' ');
                previous_space = true;
            }
        } else {
            out.push(ch);
            previous_space = false;
        }
    }
    if value.chars().count() > max_chars {
        out.push_str("...");
    }
    out.trim().to_owned()
}
