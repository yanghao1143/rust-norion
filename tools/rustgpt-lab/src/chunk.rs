pub(crate) fn answer_chunks(text: &str, max_chars: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    for character in text.chars() {
        current.push(character);
        if current.chars().count() >= max_chars
            || matches!(
                character,
                '。' | '，' | '；' | '！' | '？' | '\n' | '.' | ',' | ';'
            )
        {
            chunks.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_answer_on_chinese_punctuation() {
        let chunks = answer_chunks("你好，世界。OK", 20);
        assert_eq!(chunks, vec!["你好，", "世界。", "OK"]);
    }
}
