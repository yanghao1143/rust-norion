use std::collections::BTreeMap;

use crate::{MemoryError, MemoryResult, Metadata};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortTermEntry {
    pub key: String,
    pub value: Vec<u8>,
    pub metadata: Metadata,
    pub revision: u64,
}

impl ShortTermEntry {
    pub fn text(&self) -> Option<&str> {
        std::str::from_utf8(&self.value).ok()
    }
}

pub trait ShortTermKv {
    fn put(&mut self, key: String, value: Vec<u8>, metadata: Metadata) -> MemoryResult<()>;
    fn get(&self, key: &str) -> MemoryResult<Option<ShortTermEntry>>;
    fn delete(&mut self, key: &str) -> MemoryResult<bool>;
    fn keys(&self) -> Vec<String>;
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryShortTermKv {
    entries: BTreeMap<String, ShortTermEntry>,
    next_revision: u64,
}

impl InMemoryShortTermKv {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ShortTermKv for InMemoryShortTermKv {
    fn put(&mut self, key: String, value: Vec<u8>, metadata: Metadata) -> MemoryResult<()> {
        if key.trim().is_empty() {
            return Err(MemoryError::InvalidInput(
                "short-term key cannot be empty".to_owned(),
            ));
        }
        self.next_revision = self.next_revision.saturating_add(1);
        self.entries.insert(
            key.clone(),
            ShortTermEntry {
                key,
                value,
                metadata,
                revision: self.next_revision,
            },
        );
        Ok(())
    }

    fn get(&self, key: &str) -> MemoryResult<Option<ShortTermEntry>> {
        Ok(self.entries.get(key).cloned())
    }

    fn delete(&mut self, key: &str) -> MemoryResult<bool> {
        Ok(self.entries.remove(key).is_some())
    }

    fn keys(&self) -> Vec<String> {
        self.entries.keys().cloned().collect()
    }

    fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_term_writes_reads_and_deletes() {
        let mut kv = InMemoryShortTermKv::new();
        let mut metadata = Metadata::new();
        metadata.insert("scope".to_owned(), "turn".to_owned());

        kv.put(
            "agent:focus".to_owned(),
            b"ship memory crate".to_vec(),
            metadata,
        )
        .unwrap();

        let entry = kv.get("agent:focus").unwrap().unwrap();
        assert_eq!(entry.text(), Some("ship memory crate"));
        assert_eq!(
            entry.metadata.get("scope").map(String::as_str),
            Some("turn")
        );
        assert_eq!(kv.keys(), vec!["agent:focus".to_owned()]);
        assert!(kv.delete("agent:focus").unwrap());
        assert!(kv.is_empty());
    }
}
