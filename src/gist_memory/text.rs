pub(super) fn split_sections(answer: &str, target_chars: usize) -> Vec<String> {
    let paragraphs = answer
        .split("\n\n")
        .map(normalize_text)
        .filter(|item| item.chars().count() >= 48)
        .collect::<Vec<_>>();

    if paragraphs.len() >= 2 {
        return paragraphs;
    }

    chunk_by_chars(answer, target_chars.max(80))
        .into_iter()
        .filter(|item| item.chars().count() >= 48)
        .collect()
}

pub(super) fn split_paragraphs(answer: &str) -> Vec<String> {
    let mut paragraphs = answer
        .split('\n')
        .map(normalize_text)
        .filter(|item| item.chars().count() >= 32)
        .collect::<Vec<_>>();

    if paragraphs.len() >= 2 {
        return paragraphs;
    }

    paragraphs = split_sentences(answer)
        .into_iter()
        .filter(|item| item.chars().count() >= 32)
        .collect();

    if paragraphs.is_empty() {
        paragraphs = chunk_by_chars(answer, 180)
            .into_iter()
            .filter(|item| item.chars().count() >= 32)
            .collect();
    }

    paragraphs
}

pub(super) fn summarize_text(text: &str, max_chars: usize) -> String {
    compact(&normalize_text(text), max_chars.max(8))
}

pub(super) fn compact(text: &str, max_chars: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= max_chars {
        return text.to_owned();
    }

    let mut out = text
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    out.push_str("...");
    out
}

pub(super) fn normalize_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(super) fn estimate_tokens(text: &str) -> usize {
    if text.trim().is_empty() {
        return 0;
    }
    if text.chars().any(char::is_whitespace) {
        text.split_whitespace().count()
    } else {
        let divisor = if text.is_ascii() { 4 } else { 2 };
        text.chars().count().div_ceil(divisor).max(1)
    }
}

fn split_sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();

    for ch in text.chars() {
        current.push(ch);
        if matches!(ch, '.' | '!' | '?' | ';') {
            let sentence = normalize_text(&current);
            if !sentence.is_empty() {
                out.push(sentence);
            }
            current.clear();
        }
    }

    let tail = normalize_text(&current);
    if !tail.is_empty() {
        out.push(tail);
    }

    out
}

fn chunk_by_chars(text: &str, chunk_chars: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    let chunk_chars = chunk_chars.max(1);

    for ch in text.chars() {
        current.push(ch);
        if current.chars().count() >= chunk_chars {
            chunks.push(normalize_text(&current));
            current.clear();
        }
    }

    let tail = normalize_text(&current);
    if !tail.is_empty() {
        chunks.push(tail);
    }

    chunks
}
