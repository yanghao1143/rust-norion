use super::{GistGenerator, GistLevel};

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
