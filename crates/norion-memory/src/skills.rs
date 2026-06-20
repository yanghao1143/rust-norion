use std::collections::{BTreeMap, BTreeSet};

use crate::{MemoryError, MemoryResult, Metadata, clamp01};

#[derive(Debug, Clone, PartialEq)]
pub struct SkillRecordInput {
    pub name: String,
    pub description: String,
    pub body: String,
    pub tags: Vec<String>,
    pub metadata: Metadata,
    pub confidence: f32,
}

impl SkillRecordInput {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            body: String::new(),
            tags: Vec::new(),
            metadata: Metadata::new(),
            confidence: 0.5,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SkillRecord {
    pub id: u64,
    pub name: String,
    pub description: String,
    pub body: String,
    pub tags: Vec<String>,
    pub metadata: Metadata,
    pub confidence: f32,
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillQuery {
    pub text: String,
    pub tags: Vec<String>,
    pub limit: usize,
}

impl SkillQuery {
    pub fn new(text: impl Into<String>, limit: usize) -> Self {
        Self {
            text: text.into(),
            tags: Vec::new(),
            limit,
        }
    }
}

pub trait SkillLibrary {
    fn add_skill(&mut self, input: SkillRecordInput) -> MemoryResult<u64>;
    fn update_skill(&mut self, id: u64, input: SkillRecordInput) -> MemoryResult<bool>;
    fn get_skill(&self, id: u64) -> MemoryResult<Option<SkillRecord>>;
    fn search_skills(&self, query: SkillQuery) -> MemoryResult<Vec<SkillRecord>>;
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug, Clone)]
pub struct InMemorySkillLibrary {
    records: BTreeMap<u64, SkillRecord>,
    next_id: u64,
}

impl Default for InMemorySkillLibrary {
    fn default() -> Self {
        Self {
            records: BTreeMap::new(),
            next_id: 1,
        }
    }
}

impl InMemorySkillLibrary {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SkillLibrary for InMemorySkillLibrary {
    fn add_skill(&mut self, input: SkillRecordInput) -> MemoryResult<u64> {
        validate_skill_input(&input)?;
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.records.insert(id, skill_from_input(id, 1, input));
        Ok(id)
    }

    fn update_skill(&mut self, id: u64, input: SkillRecordInput) -> MemoryResult<bool> {
        validate_skill_input(&input)?;
        let Some(previous) = self.records.get(&id) else {
            return Ok(false);
        };
        let version = previous.version.saturating_add(1);
        self.records
            .insert(id, skill_from_input(id, version, input));
        Ok(true)
    }

    fn get_skill(&self, id: u64) -> MemoryResult<Option<SkillRecord>> {
        Ok(self.records.get(&id).cloned())
    }

    fn search_skills(&self, query: SkillQuery) -> MemoryResult<Vec<SkillRecord>> {
        let limit = query.limit.max(1);
        let query_tags = query
            .tags
            .iter()
            .map(|tag| tag.to_ascii_lowercase())
            .collect::<BTreeSet<_>>();
        let query_tokens = tokens(&query.text);
        let mut matches = self
            .records
            .values()
            .filter_map(|skill| {
                let tag_score = if query_tags.is_empty() {
                    0.0
                } else {
                    let skill_tags = skill
                        .tags
                        .iter()
                        .map(|tag| tag.to_ascii_lowercase())
                        .collect::<BTreeSet<_>>();
                    query_tags.intersection(&skill_tags).count() as f32 / query_tags.len() as f32
                };
                let text = format!("{} {} {}", skill.name, skill.description, skill.body);
                let text_score = overlap(&query_tokens, &tokens(&text));
                let score = (tag_score * 0.46 + text_score * 0.44 + skill.confidence * 0.10)
                    .clamp(0.0, 1.0);
                (score > 0.0).then_some((score, skill.clone()))
            })
            .collect::<Vec<_>>();
        matches.sort_by(|left, right| {
            right
                .0
                .partial_cmp(&left.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.1.id.cmp(&right.1.id))
        });
        Ok(matches
            .into_iter()
            .take(limit)
            .map(|(_, skill)| skill)
            .collect())
    }

    fn len(&self) -> usize {
        self.records.len()
    }
}

fn validate_skill_input(input: &SkillRecordInput) -> MemoryResult<()> {
    if input.name.trim().is_empty() {
        return Err(MemoryError::InvalidInput(
            "skill name cannot be empty".to_owned(),
        ));
    }
    if input.description.trim().is_empty() {
        return Err(MemoryError::InvalidInput(
            "skill description cannot be empty".to_owned(),
        ));
    }
    Ok(())
}

fn skill_from_input(id: u64, version: u64, input: SkillRecordInput) -> SkillRecord {
    SkillRecord {
        id,
        name: input.name,
        description: input.description,
        body: input.body,
        tags: input.tags,
        metadata: input.metadata,
        confidence: clamp01(input.confidence),
        version,
    }
}

fn tokens(value: &str) -> BTreeSet<String> {
    value
        .split(|ch: char| ch.is_whitespace() || ch.is_ascii_punctuation())
        .filter(|token| !token.trim().is_empty())
        .map(|token| token.to_ascii_lowercase())
        .collect()
}

fn overlap(left: &BTreeSet<String>, right: &BTreeSet<String>) -> f32 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    left.intersection(right).count() as f32 / left.len().min(right.len()) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_library_writes_reads_updates_and_searches() {
        let mut library = InMemorySkillLibrary::new();
        let mut input = SkillRecordInput::new("rust-check", "Compile generated Rust snippets");
        input.tags = vec!["rust".to_owned(), "validation".to_owned()];
        input.body = "Use rustc metadata mode before reinforcing generated code.".to_owned();

        let id = library.add_skill(input).unwrap();
        assert_eq!(library.get_skill(id).unwrap().unwrap().version, 1);

        let matches = library
            .search_skills(SkillQuery {
                text: "generated rust validation".to_owned(),
                tags: vec!["rust".to_owned()],
                limit: 5,
            })
            .unwrap();
        assert_eq!(matches[0].id, id);

        let mut updated = SkillRecordInput::new("rust-check", "Compile generated Rust snippets");
        updated.body = "Also capture diagnostics for feedback.".to_owned();
        assert!(library.update_skill(id, updated).unwrap());
        assert_eq!(library.get_skill(id).unwrap().unwrap().version, 2);
    }
}
