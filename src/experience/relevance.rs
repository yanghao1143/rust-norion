use std::collections::HashSet;

use super::text_normalize::normalize_full_width_ascii_char;

const TASK_PHRASE_ANCHORS: &[(&str, &[&str])] = &[
    ("代码", &["code"]),
    ("循环", &["loop"]),
    ("输出", &["output", "println", "print"]),
    ("函数", &["function", "fn"]),
    ("编译", &["compile", "compiler"]),
    ("错误", &["error"]),
    ("接口", &["interface", "trait"]),
    ("后端", &["backend"]),
    ("前端", &["frontend", "ui"]),
    ("模型", &["model"]),
    ("测试", &["test"]),
    ("部署", &["deploy"]),
    ("索引", &["index"]),
    ("上下文", &["context"]),
    ("流式", &["stream"]),
    ("业务", &["business"]),
    ("联调", &["integration"]),
];

pub(super) fn lexical_overlap(left: &str, right: &str) -> f32 {
    let left_chars = left
        .chars()
        .map(normalize_full_width_ascii_char)
        .flat_map(char::to_lowercase)
        .filter(|ch| is_signal_char(*ch))
        .collect::<HashSet<_>>();
    let right_chars = right
        .chars()
        .map(normalize_full_width_ascii_char)
        .flat_map(char::to_lowercase)
        .filter(|ch| is_signal_char(*ch))
        .collect::<HashSet<_>>();

    if left_chars.is_empty() || right_chars.is_empty() {
        return 0.0;
    }

    let shared = left_chars.intersection(&right_chars).count() as f32;
    let denom = left_chars.len().min(right_chars.len()) as f32;
    (shared / denom).clamp(0.0, 1.0)
}

pub(super) fn is_signal_char(ch: char) -> bool {
    let ch = normalize_full_width_ascii_char(ch);
    !ch.is_whitespace() && !ch.is_ascii_punctuation() && !is_cjk_punctuation(ch)
}

pub(super) fn is_cjk_punctuation(ch: char) -> bool {
    matches!(
        ch,
        '，' | '。'
            | '、'
            | '；'
            | '：'
            | '！'
            | '？'
            | '（'
            | '）'
            | '【'
            | '】'
            | '《'
            | '》'
            | '“'
            | '”'
            | '‘'
            | '’'
            | '—'
            | '…'
    )
}

pub(super) fn task_anchor_penalty(prompt: &str, signal_text: &str) -> f32 {
    let anchors = task_anchor_terms(prompt);
    if anchors.len() < 2 {
        return 0.0;
    }

    let signal_words = ascii_word_set(signal_text);
    let signal_ascii_lower = signal_text.to_ascii_lowercase();
    let matched = anchors
        .iter()
        .filter(|anchor| {
            if anchor.is_ascii() {
                signal_words.contains(anchor.as_str())
            } else {
                signal_ascii_lower.contains(anchor.as_str())
            }
        })
        .count();
    let coverage = matched as f32 / anchors.len() as f32;

    let phrase_penalty = phrase_anchor_penalty(prompt, &signal_words, &signal_ascii_lower);
    let base_penalty = if coverage <= 0.0 {
        0.56
    } else if coverage < 0.34 {
        0.26
    } else if coverage < 0.50 {
        0.04
    } else {
        0.0
    };
    base_penalty + phrase_penalty
}

fn task_anchor_terms(prompt: &str) -> Vec<String> {
    let mut terms = ascii_terms(prompt);
    for (phrase, _) in TASK_PHRASE_ANCHORS {
        if prompt.contains(phrase) {
            terms.push((*phrase).to_owned());
        }
    }
    terms.sort();
    terms.dedup();
    terms
}

fn phrase_anchor_penalty(
    prompt: &str,
    signal_words: &HashSet<String>,
    signal_ascii_lower: &str,
) -> f32 {
    let required = TASK_PHRASE_ANCHORS
        .iter()
        .filter(|(phrase, _)| prompt.contains(phrase))
        .collect::<Vec<_>>();
    if required.len() < 2 {
        return 0.0;
    }

    let matched = required
        .iter()
        .filter(|(phrase, aliases)| {
            signal_ascii_lower.contains(*phrase)
                || aliases
                    .iter()
                    .any(|alias| alias_matches_signal(alias, signal_words, signal_ascii_lower))
        })
        .count();

    if matched == 0 {
        0.18
    } else if matched * 2 < required.len() {
        0.10
    } else {
        0.0
    }
}

fn alias_matches_signal(
    alias: &str,
    signal_words: &HashSet<String>,
    signal_ascii_lower: &str,
) -> bool {
    signal_words.contains(alias)
        || (alias.chars().count() >= 3 && signal_ascii_lower.contains(alias))
}

fn ascii_terms(value: &str) -> Vec<String> {
    let mut terms = Vec::new();
    let mut current = String::new();
    for ch in value.chars() {
        let ch = normalize_full_width_ascii_char(ch);
        if ch.is_ascii_alphanumeric() {
            current.push(ch.to_ascii_lowercase());
        } else if !current.is_empty() {
            push_ascii_term(&mut terms, &current);
            current.clear();
        }
    }
    if !current.is_empty() {
        push_ascii_term(&mut terms, &current);
    }
    terms
}

fn ascii_word_set(value: &str) -> HashSet<String> {
    ascii_terms(value).into_iter().collect()
}

fn push_ascii_term(terms: &mut Vec<String>, term: &str) {
    if term.chars().count() >= 3 {
        terms.push(term.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexical_overlap_ignores_cjk_punctuation() {
        assert_eq!(
            lexical_overlap("保持提示门控，摘要路由证据。", "保持提示门控摘要路由证据"),
            1.0
        );
    }

    #[test]
    fn lexical_overlap_and_task_anchors_accept_full_width_ascii() {
        assert_eq!(lexical_overlap("Ｒｕｓｔ ｌｏｏｐ", "rust loop"), 1.0);
        assert_eq!(
            task_anchor_penalty(
                "please write Ｒｕｓｔ ｌｏｏｐ ｏｕｔｐｕｔ",
                "rust loop println output"
            ),
            0.0
        );
    }
}
