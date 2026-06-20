use super::model::{GistLevel, GistRecord};
use super::text::{
    compact, estimate_tokens, normalize_text, split_paragraphs, split_sections, summarize_text,
};

#[derive(Debug, Clone)]
pub struct GistGenerator {
    pub(super) min_quality: f32,
    pub(super) min_source_chars: usize,
    pub(super) max_summary_chars: usize,
    pub(super) max_sections: usize,
    pub(super) max_paragraphs: usize,
    pub(super) section_chars: usize,
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
