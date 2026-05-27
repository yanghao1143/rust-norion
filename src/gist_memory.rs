#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GistLevel {
    Document,
    Section,
    Paragraph,
}

impl GistLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::Section => "section",
            Self::Paragraph => "paragraph",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "document" => Some(Self::Document),
            "section" => Some(Self::Section),
            "paragraph" => Some(Self::Paragraph),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GistRecord {
    pub level: GistLevel,
    pub title: String,
    pub summary: String,
    pub source_tokens: usize,
    pub importance: f32,
}

impl GistRecord {
    pub fn hint(&self) -> String {
        format!(
            "{}:{} importance={:.3} tokens={} summary={}",
            self.level.as_str(),
            self.title,
            self.importance,
            self.source_tokens,
            self.summary
        )
    }
}

#[derive(Debug, Clone)]
pub struct GistGenerator {
    min_quality: f32,
    min_source_chars: usize,
    max_summary_chars: usize,
    max_sections: usize,
    max_paragraphs: usize,
    section_chars: usize,
}

impl Default for GistGenerator {
    fn default() -> Self {
        Self {
            min_quality: 0.50,
            min_source_chars: 96,
            max_summary_chars: 180,
            max_sections: 3,
            max_paragraphs: 3,
            section_chars: 420,
        }
    }
}

impl GistGenerator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_min_quality(mut self, min_quality: f32) -> Self {
        self.min_quality = min_quality.clamp(0.0, 1.0);
        self
    }

    pub fn generate(&self, prompt: &str, answer: &str, quality: f32) -> Vec<GistRecord> {
        let quality = quality.clamp(0.0, 1.0);
        let prompt = normalize_text(prompt);
        let answer = normalize_text(answer);
        let combined_chars = prompt.chars().count() + answer.chars().count();

        if quality < self.min_quality || combined_chars < self.min_source_chars {
            return Vec::new();
        }

        let mut records = Vec::new();
        let document_source = format!("{prompt} {answer}");
        records.push(GistRecord {
            level: GistLevel::Document,
            title: compact(&prompt, 64),
            summary: summarize_text(&answer, self.max_summary_chars),
            source_tokens: estimate_tokens(&document_source),
            importance: importance(GistLevel::Document, quality, 0),
        });

        for (index, section) in split_sections(&answer, self.section_chars)
            .into_iter()
            .take(self.max_sections)
            .enumerate()
        {
            records.push(GistRecord {
                level: GistLevel::Section,
                title: format!("section-{}:{}", index + 1, compact(&section, 48)),
                summary: summarize_text(&section, self.max_summary_chars),
                source_tokens: estimate_tokens(&section),
                importance: importance(GistLevel::Section, quality, index),
            });
        }

        for (index, paragraph) in split_paragraphs(&answer)
            .into_iter()
            .take(self.max_paragraphs)
            .enumerate()
        {
            records.push(GistRecord {
                level: GistLevel::Paragraph,
                title: format!("paragraph-{}:{}", index + 1, compact(&paragraph, 48)),
                summary: summarize_text(&paragraph, self.max_summary_chars),
                source_tokens: estimate_tokens(&paragraph),
                importance: importance(GistLevel::Paragraph, quality, index),
            });
        }

        dedupe_records(records)
    }
}

fn split_sections(answer: &str, target_chars: usize) -> Vec<String> {
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

fn split_paragraphs(answer: &str) -> Vec<String> {
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

fn dedupe_records(records: Vec<GistRecord>) -> Vec<GistRecord> {
    let mut out: Vec<GistRecord> = Vec::new();

    for record in records {
        let duplicate = out
            .iter()
            .any(|existing| existing.level == record.level && existing.summary == record.summary);
        if !duplicate {
            out.push(record);
        }
    }

    out
}

fn importance(level: GistLevel, quality: f32, index: usize) -> f32 {
    let base = match level {
        GistLevel::Document => 0.48,
        GistLevel::Section => 0.38,
        GistLevel::Paragraph => 0.28,
    };
    (base + quality * 0.48 - index as f32 * 0.04).clamp(0.0, 1.0)
}

fn summarize_text(text: &str, max_chars: usize) -> String {
    compact(&normalize_text(text), max_chars.max(8))
}

fn compact(text: &str, max_chars: usize) -> String {
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

fn normalize_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn estimate_tokens(text: &str) -> usize {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_quality_text_produces_hierarchical_gists() {
        let generator = GistGenerator::new().with_min_quality(0.1);
        let answer = "\
            The router uses entropy and task pressure to choose compute paths. \
            It keeps simple tokens on projection and sends hard tokens to attention.\n\n\
            The memory layer stores durable KV summaries on disk. \
            It keeps hot memories small and promotes high quality records.";

        let records = generator.generate("Rust Noiron long context", answer, 0.9);

        assert!(
            records
                .iter()
                .any(|record| record.level == GistLevel::Document)
        );
        assert!(
            records
                .iter()
                .any(|record| record.level == GistLevel::Section)
        );
        assert!(
            records
                .iter()
                .any(|record| record.level == GistLevel::Paragraph)
        );
        assert!(records.iter().all(|record| record.importance > 0.0));
    }

    #[test]
    fn low_quality_text_is_not_admitted() {
        let generator = GistGenerator::new();

        let records = generator.generate("prompt", "short answer", 0.2);

        assert!(records.is_empty());
    }

    #[test]
    fn summaries_are_bounded() {
        let generator = GistGenerator {
            max_summary_chars: 32,
            min_source_chars: 16,
            ..GistGenerator::new().with_min_quality(0.1)
        };
        let answer = "a ".repeat(100);

        let records = generator.generate("prompt with enough context", &answer, 0.9);

        assert!(!records.is_empty());
        assert!(
            records
                .iter()
                .all(|record| record.summary.chars().count() <= 32)
        );
    }
}
