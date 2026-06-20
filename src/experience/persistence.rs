use std::collections::HashSet;
use std::io;
use std::path::Path;

use crate::disk_kv::DiskKvStore;

use super::ExperienceStore;
use super::codec::{deserialize_record, serialize_record};
use super::hygiene;

impl ExperienceStore {
    pub fn save_to_disk_kv(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let mut store = DiskKvStore::open(path)?;
        let mut live_keys = HashSet::new();

        for record in self
            .records
            .iter()
            .filter(|record| hygiene::admission_persistence_block(record).is_none())
        {
            let key = format!("experience/{}", record.id);
            live_keys.insert(key.clone());
            store.put(&key, serialize_record(record).as_bytes())?;
        }

        for stale_key in store.keys_with_prefix("experience/") {
            if !live_keys.contains(&stale_key) {
                store.delete(&stale_key)?;
            }
        }

        store.put(
            "meta/next_experience_id",
            self.next_id.to_string().as_bytes(),
        )?;
        store.compact()
    }

    pub fn load_from_disk_kv(path: impl AsRef<Path>) -> io::Result<Self> {
        let store = DiskKvStore::open(path)?;
        let mut out = Self::new();

        for key in store.keys_with_prefix("experience/") {
            let Some(value) = store.get(&key)? else {
                continue;
            };
            let Ok(line) = String::from_utf8(value) else {
                continue;
            };
            let Some(record) = deserialize_record(&line) else {
                continue;
            };
            out.next_id = out.next_id.max(record.id + 1);
            out.records.push(record);
        }

        out.records.sort_by_key(|record| record.id);
        if let Some(value) = store.get("meta/next_experience_id")?
            && let Ok(next_id) = String::from_utf8_lossy(&value).parse::<u64>()
        {
            out.next_id = out.next_id.max(next_id);
        }

        Ok(out)
    }
}
