use std::collections::HashSet;

use super::model::ExperienceRecord;
use super::relevance::is_signal_char;
use super::text_normalize::{normalize_full_width_ascii, normalized_marker_span};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ExperienceRetrievalNoise {
    pub prompt_transcript_like: bool,
    pub lesson_transcript_like: bool,
    pub metadata_lesson_like: bool,
    pub clean_lesson_like: bool,
    pub has_clean_gist: bool,
}

impl ExperienceRetrievalNoise {
    pub fn suppress_prompt_index(self) -> bool {
        self.prompt_transcript_like
            && (self.metadata_lesson_like || (!self.has_clean_gist && !self.clean_lesson_like))
    }

    pub fn effective_quality_multiplier(self) -> f32 {
        if self.metadata_lesson_like && !self.has_clean_gist {
            0.35
        } else if self.metadata_lesson_like {
            0.82
        } else if self.prompt_transcript_like && !self.clean_lesson_like && !self.has_clean_gist {
            0.62
        } else {
            1.0
        }
    }

    pub fn penalty(self) -> f32 {
        let mut penalty = 0.0;
        if self.prompt_transcript_like && !self.has_clean_gist {
            penalty += if self.clean_lesson_like { 0.06 } else { 0.16 };
        }
        if self.lesson_transcript_like {
            penalty += 0.22;
        }
        if self.metadata_lesson_like {
            penalty += if self.has_clean_gist { 0.07 } else { 0.28 };
        }
        if self.prompt_transcript_like
            && !self.clean_lesson_like
            && !self.has_clean_gist
            && !self.metadata_lesson_like
        {
            penalty += 0.10;
        }
        penalty
    }
}

pub(super) fn transcript_anchor_penalty(record_prompt: &str, current_prompt: &str) -> f32 {
    if !text_has_transcript_shape(record_prompt) || user_turn_count(record_prompt) <= 1 {
        return 0.0;
    }

    let Some(first_user) = first_user_turn(record_prompt) else {
        return 0.0;
    };
    if lexical_char_overlap(current_prompt, first_user) >= 0.24 {
        return 0.0;
    }

    let first_user = first_user.trim();
    if is_generic_chat_opening(first_user) {
        0.13
    } else {
        0.08
    }
}

pub(super) fn retrieval_noise(record: &ExperienceRecord) -> ExperienceRetrievalNoise {
    ExperienceRetrievalNoise {
        prompt_transcript_like: text_has_transcript_shape(&record.prompt),
        lesson_transcript_like: text_has_transcript_shape(&record.lesson)
            || text_has_role_labeled_lesson_residue(&record.lesson),
        metadata_lesson_like: text_has_metadata_lesson_shape(&record.lesson),
        clean_lesson_like: text_has_clean_lesson_shape(&record.lesson),
        has_clean_gist: record.gist_records.iter().any(|gist| {
            text_has_clean_lesson_shape(&gist.summary)
                && !text_has_metadata_lesson_shape(&gist.summary)
        }),
    }
}

pub(super) fn text_has_transcript_shape(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    contains_labeled_prefix(&value, "conversation transcript")
        || (contains_labeled_prefix(&value, "user") && contains_labeled_prefix(&value, "assistant"))
}

pub(super) fn text_has_role_labeled_lesson_residue(value: &str) -> bool {
    let value = strip_repeated_generated_prefixes(value.trim_start());
    strip_role_label(value, "assistant").is_some() || strip_role_label(value, "user").is_some()
}

pub(super) fn text_has_metadata_lesson_shape(value: &str) -> bool {
    let value =
        normalize_full_width_ascii(strip_reusable_text_prefixes(value.trim())).to_ascii_lowercase();
    metadata_lesson_label(&value).is_some()
        || ((value.contains("quality=") || value.contains("overlap="))
            && value.contains("max_severity="))
}

pub(super) fn text_has_rejected_metadata_lesson_shape(value: &str) -> bool {
    let value =
        normalize_full_width_ascii(strip_reusable_text_prefixes(value.trim())).to_ascii_lowercase();
    metadata_lesson_label(&value) == Some("rejected_pattern")
}

fn metadata_lesson_label(value: &str) -> Option<&'static str> {
    for label in ["accepted_pattern", "rejected_pattern"] {
        if strip_prefixed_label(value, label, &[" ", ":", "："]).is_some() {
            return Some(label);
        }
    }
    None
}

pub(super) fn strip_reusable_text_prefixes(value: &str) -> &str {
    let mut current = value.trim_start();
    for _ in 0..4 {
        let before_len = current.len();
        current = strip_response_lesson_prefix(current).unwrap_or(current);
        current = strip_generated_prefixes(current);
        current = strip_lesson_role_label(current);
        if current.len() == before_len {
            break;
        }
    }
    current
}

fn strip_generated_prefixes(value: &str) -> &str {
    strip_prefixed_label(value, "asthought", &[" ", ":", "："])
        .unwrap_or_else(|| value.trim_start())
}

pub(super) fn strip_response_lesson_prefix(value: &str) -> Option<&str> {
    for prefix in ["reuse_response", "revise_response"] {
        if let Some(stripped) = strip_prefixed_label(value, prefix, &[":", "："]) {
            return Some(stripped);
        }
    }
    None
}

pub(super) fn strip_reuse_response_prefix(value: &str) -> Option<&str> {
    strip_prefixed_label(value, "reuse_response", &[":", "："])
}

pub(super) fn strip_at_case_insensitive_marker<'a>(value: &'a str, marker: &str) -> &'a str {
    let full_width_colon_marker = marker.replace(':', "：");
    let ascii_index = normalized_marker_span(value, marker).map(|(index, _)| index);
    let full_width_index = (full_width_colon_marker != marker)
        .then(|| normalized_marker_span(value, &full_width_colon_marker).map(|(index, _)| index))
        .flatten();
    [ascii_index, full_width_index]
        .into_iter()
        .flatten()
        .min()
        .and_then(|index| value.get(..index))
        .unwrap_or(value)
        .trim()
}

pub(super) fn strip_reflection_lesson_suffix(value: &str) -> &str {
    let body = strip_at_case_insensitive_marker(value, " [reflection ");
    strip_at_case_insensitive_marker(body, " reflection repair:")
}

fn strip_repeated_generated_prefixes(value: &str) -> &str {
    let mut current = value.trim_start();
    for _ in 0..4 {
        let before_len = current.len();
        current = strip_generated_prefixes(current);
        if current.len() == before_len {
            break;
        }
    }
    current
}

fn strip_lesson_role_label(value: &str) -> &str {
    for role in ["assistant", "user"] {
        if let Some(stripped) = strip_role_label(value, role) {
            return stripped;
        }
    }
    value.trim_start()
}

fn text_has_clean_lesson_shape(value: &str) -> bool {
    let trimmed = strip_reusable_text_prefixes(value.trim());
    if trimmed.is_empty()
        || trimmed.chars().count() > 420
        || text_has_transcript_shape(trimmed)
        || text_has_metadata_lesson_shape(trimmed)
    {
        return false;
    }

    trimmed
        .chars()
        .filter(|ch| is_signal_char(*ch))
        .take(12)
        .count()
        >= 12
}

fn user_turn_count(value: &str) -> usize {
    normalized_marker_count(value, "user:")
}

fn first_user_turn(value: &str) -> Option<&str> {
    let lower = value.to_ascii_lowercase();
    let user_start = first_labeled_prefix(&lower, "user")?;
    let after_user = value.get(user_start..)?;
    let after_user_lower = lower.get(user_start..)?;
    let end = next_role_turn_boundary(after_user_lower).unwrap_or(after_user.len());
    after_user.get(..end)
}

fn contains_labeled_prefix(value: &str, label: &str) -> bool {
    normalized_marker_span(value, &format!("{label}:")).is_some()
}

fn first_labeled_prefix(value: &str, label: &str) -> Option<usize> {
    normalized_marker_span(value, &format!("{label}:")).map(|(_, end)| end)
}

fn next_role_turn_boundary(value: &str) -> Option<usize> {
    [
        "\nassistant:",
        "\nassistant：",
        "\r\nassistant:",
        "\r\nassistant：",
        "\nuser:",
        "\nuser：",
        "\r\nuser:",
        "\r\nuser：",
    ]
    .into_iter()
    .filter_map(|marker| normalized_marker_span(value, marker).map(|(index, _)| index))
    .min()
}

fn strip_role_label<'a>(value: &'a str, role: &str) -> Option<&'a str> {
    strip_prefixed_label(value, role, &[":", "："])
}

fn strip_prefixed_label<'a>(value: &'a str, label: &str, delimiters: &[&str]) -> Option<&'a str> {
    let trimmed = value.trim_start();
    let rest = strip_normalized_prefix(trimmed, label)?;
    for delimiter in delimiters {
        if let Some(stripped) = strip_normalized_prefix(rest, delimiter) {
            return Some(stripped.trim_start());
        }
    }
    None
}

fn lexical_char_overlap(left: &str, right: &str) -> f32 {
    let left_chars = normalized_chars(left);
    let right_chars = normalized_chars(right);
    if left_chars.is_empty() || right_chars.is_empty() {
        return 0.0;
    }

    let shared = left_chars.intersection(&right_chars).count() as f32;
    let denom = left_chars.len().min(right_chars.len()) as f32;
    (shared / denom).clamp(0.0, 1.0)
}

fn normalized_chars(value: &str) -> HashSet<char> {
    value
        .chars()
        .flat_map(char::to_lowercase)
        .filter(|ch| is_signal_char(*ch))
        .collect()
}

fn strip_normalized_prefix<'a>(value: &'a str, prefix: &str) -> Option<&'a str> {
    let (start, end) = normalized_marker_span(value, prefix)?;
    (start == 0).then(|| value.get(end..)).flatten()
}

fn normalized_marker_count(value: &str, marker: &str) -> usize {
    let mut count = 0usize;
    let mut remaining = value;
    while let Some((_, end)) = normalized_marker_span(remaining, marker) {
        count = count.saturating_add(1);
        remaining = &remaining[end..];
    }
    count
}

fn is_generic_chat_opening(value: &str) -> bool {
    let value = value.trim().to_ascii_lowercase();
    matches!(
        value.as_str(),
        "hi" | "hello" | "hey" | "你好" | "您好" | "在吗" | "你会什么"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_reusable_text_prefixes_in_repeated_orders() {
        assert_eq!(
            strip_reusable_text_prefixes("assistant: AsThought: Use compact route evidence"),
            "Use compact route evidence"
        );
        assert_eq!(
            strip_reusable_text_prefixes("AsThought: user: Use compact route evidence"),
            "Use compact route evidence"
        );
        assert_eq!(
            strip_reusable_text_prefixes("Reuse_Response: assistant: Use compact route evidence"),
            "Use compact route evidence"
        );
    }

    #[test]
    fn transcript_shape_accepts_full_width_role_delimiters() {
        assert!(text_has_transcript_shape(
            "Conversation transcript：\nuser：你好\nassistant：可以继续"
        ));
        assert!(text_has_transcript_shape(
            "user：请总结经验\nassistant：保留紧凑 route evidence"
        ));
        assert_eq!(
            user_turn_count("user：第一轮\nassistant：ok\nuser: 第二轮"),
            2
        );
        assert_eq!(
            first_user_turn("Conversation transcript：\nuser：你好\nassistant：可以继续")
                .map(str::trim),
            Some("你好")
        );
    }

    #[test]
    fn role_labeled_prefix_stripping_accepts_full_width_colon() {
        assert!(text_has_role_labeled_lesson_residue(
            "AsThought: assistant： prefer compact route evidence"
        ));
        assert!(text_has_role_labeled_lesson_residue(
            "ASTHOUGHT user： prefer compact route evidence"
        ));
        assert_eq!(
            strip_reusable_text_prefixes("Reuse_Response: assistant： Use compact route evidence"),
            "Use compact route evidence"
        );
        assert_eq!(
            strip_reusable_text_prefixes("Reuse_Response： assistant： Use compact route evidence"),
            "Use compact route evidence"
        );
        assert_eq!(
            strip_reusable_text_prefixes("AsThought: user： Use compact route evidence"),
            "Use compact route evidence"
        );
        assert_eq!(
            strip_reusable_text_prefixes("AsThought： user： Use compact route evidence"),
            "Use compact route evidence"
        );
    }

    #[test]
    fn reusable_prefix_stripping_accepts_full_width_ascii_labels() {
        assert_eq!(
            strip_reusable_text_prefixes(
                "Ｒｅｕｓｅ＿Ｒｅｓｐｏｎｓｅ： ａｓｓｉｓｔａｎｔ： Use compact route evidence"
            ),
            "Use compact route evidence"
        );
        assert_eq!(
            strip_reusable_text_prefixes(
                "ＡｓＴｈｏｕｇｈｔ： ｕｓｅｒ： Keep route evidence compact"
            ),
            "Keep route evidence compact"
        );
        assert!(text_has_metadata_lesson_shape(
            "ＡｓＴｈｏｕｇｈｔ： ｕｓｅｒ： ｒｅｊｅｃｔｅｄ＿ｐａｔｔｅｒｎ："
        ));
        assert!(text_has_rejected_metadata_lesson_shape(
            "ＡｓＴｈｏｕｇｈｔ： ｕｓｅｒ： ｒｅｊｅｃｔｅｄ＿ｐａｔｔｅｒｎ："
        ));
        assert!(text_has_transcript_shape(
            "Ｃｏｎｖｅｒｓａｔｉｏｎ ｔｒａｎｓｃｒｉｐｔ：\nｕｓｅｒ：你好\nａｓｓｉｓｔａｎｔ：可以继续"
        ));
        assert_eq!(
            user_turn_count(
                "Ｃｏｎｖｅｒｓａｔｉｏｎ ｔｒａｎｓｃｒｉｐｔ：\nｕｓｅｒ：你好\nａｓｓｉｓｔａｎｔ：可以继续\nｕｓｅｒ：写一个 Rust 循环"
            ),
            2
        );
        assert_eq!(
            first_user_turn(
                "Ｃｏｎｖｅｒｓａｔｉｏｎ ｔｒａｎｓｃｒｉｐｔ：\nｕｓｅｒ：你好\nａｓｓｉｓｔａｎｔ：可以继续"
            )
            .map(str::trim),
            Some("你好")
        );
        assert!(
            transcript_anchor_penalty(
                "Ｃｏｎｖｅｒｓａｔｉｏｎ ｔｒａｎｓｃｒｉｐｔ：\nｕｓｅｒ：你好\nａｓｓｉｓｔａｎｔ：可以继续\nｕｓｅｒ：写一个 Rust 循环",
                "请实现命令行参数解析"
            ) > 0.0
        );
    }

    #[test]
    fn clean_lesson_shape_uses_prefix_stripped_body() {
        assert!(text_has_clean_lesson_shape(
            "assistant: AsThought: Use compact route evidence before retrying slow Gemma checks"
        ));
        assert!(!text_has_clean_lesson_shape(
            "assistant: AsThought: accepted_pattern quality=0.9 overlap=0.8 max_severity=info"
        ));
        assert!(text_has_clean_lesson_shape(
            "Reuse_Response: assistant: Use compact route evidence before retrying slow Gemma checks"
        ));
        assert!(!text_has_clean_lesson_shape(
            "Reuse_Response: assistant: accepted_pattern quality=0.9 overlap=0.8 max_severity=info"
        ));
    }

    #[test]
    fn metadata_lesson_shape_uses_prefix_stripped_body() {
        assert!(text_has_metadata_lesson_shape(
            "assistant: AsThought: accepted_pattern quality=0.9 overlap=0.8 max_severity=info"
        ));
        assert!(text_has_metadata_lesson_shape(
            "AsThought: user: rejected_pattern quality=0.1 issues=1 max_severity=critical"
        ));
        assert!(text_has_metadata_lesson_shape(
            "Reuse_Response: assistant: accepted_pattern quality=0.9 overlap=0.8 max_severity=info"
        ));
        assert!(text_has_metadata_lesson_shape(
            "Reuse_Response： assistant： accepted_pattern quality=0.9 overlap=0.8 max_severity=info"
        ));
        assert!(text_has_metadata_lesson_shape(
            "Reuse_Response： assistant： accepted_pattern："
        ));
        assert!(text_has_metadata_lesson_shape(
            "AsThought： user： rejected_pattern："
        ));
        assert!(text_has_metadata_lesson_shape(
            "ＡｓＴｈｏｕｇｈｔ： ｕｓｅｒ： ｑｕａｌｉｔｙ＝０．９ ｏｖｅｒｌａｐ＝０．８ ｍａｘ＿ｓｅｖｅｒｉｔｙ＝ｉｎｆｏ"
        ));
        assert!(text_has_rejected_metadata_lesson_shape(
            "AsThought： user： rejected_pattern："
        ));
        assert!(text_has_rejected_metadata_lesson_shape(
            "ＡｓＴｈｏｕｇｈｔ： ｕｓｅｒ： ｒｅｊｅｃｔｅｄ＿ｐａｔｔｅｒｎ："
        ));
        assert!(!text_has_rejected_metadata_lesson_shape(
            "Reuse_Response： assistant： accepted_pattern："
        ));
    }

    #[test]
    fn role_labeled_lesson_residue_ignores_generated_prefix() {
        assert!(text_has_role_labeled_lesson_residue(
            "AsThought: assistant: prefer compact route evidence"
        ));
        assert!(text_has_role_labeled_lesson_residue(
            "ASTHOUGHT user: prefer compact route evidence"
        ));
        assert!(text_has_role_labeled_lesson_residue(
            "AsThought: AsThought: assistant: prefer compact route evidence"
        ));
    }

    #[test]
    fn strips_reflection_lesson_suffixes_case_insensitively() {
        assert_eq!(
            strip_reflection_lesson_suffix(
                "Use compact route evidence [Reflection accepted q=0.842]"
            ),
            "Use compact route evidence"
        );
        assert_eq!(
            strip_reflection_lesson_suffix(
                "Use compact route evidence Reflection repair: keep only route evidence"
            ),
            "Use compact route evidence"
        );
        assert_eq!(
            strip_reflection_lesson_suffix(
                "Use compact route evidence reflection repair: keep only route evidence"
            ),
            "Use compact route evidence"
        );
        assert_eq!(
            strip_reflection_lesson_suffix(
                "Use compact route evidence Reflection repair： keep only route evidence"
            ),
            "Use compact route evidence"
        );
    }
}
