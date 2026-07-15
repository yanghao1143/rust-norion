use std::collections::HashSet;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::Path;

use crate::disk_kv::DiskKvStore;
use crate::kv_quant::QuantizationBits;

use super::cache::KvFusionCache;
use super::codec::{
    deserialize_entry, legacy_backup_path, serialize_entry, serialize_entry_quantized,
};

impl KvFusionCache {
    pub fn save_to_disk(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let mut content = String::new();
        content.push_str("# noiron-kv-cache-v1\n");

        for entry in &self.entries {
            content.push_str(&serialize_entry(entry));
            content.push('\n');
        }

        fs::write(path, content)
    }

    pub fn load_from_disk(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::new());
        }

        let content = fs::read_to_string(path)?;
        let mut cache = Self::new();
        let mut seen_ids = HashSet::new();

        for line in content.lines().filter(|line| !line.starts_with('#')) {
            let Some(entry) = deserialize_entry(line) else {
                continue;
            };
            let id = entry.id;
            if !seen_ids.insert(id) {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    format!("duplicate memory id in legacy cache: {id}"),
                ));
            }
            cache.clock = cache.clock.max(entry.created_at).max(entry.last_access);
            cache.entries.push(entry);
            cache.next_id = cache.next_id.max(id + 1);
        }
        cache.clock = cache.clock.saturating_add(1);

        Ok(cache)
    }

    pub fn save_persistent(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let path = path.as_ref();

        match self.save_to_disk_kv(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::InvalidData && path.exists() => {
                let backup_path = legacy_backup_path(path);
                fs::rename(path, &backup_path)?;
                self.save_to_disk_kv(path)
            }
            Err(error) => Err(error),
        }
    }

    pub fn load_persistent(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();

        match Self::load_from_disk_kv(path) {
            Ok(cache) => Ok(cache),
            Err(error) if error.kind() == ErrorKind::InvalidData => Self::load_from_disk(path),
            Err(error) => Err(error),
        }
    }

    pub fn save_to_disk_kv(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let mut store = DiskKvStore::open(path)?;
        let mut live_keys = HashSet::new();

        for entry in &self.entries {
            let key = format!("memory/{}", entry.id);
            live_keys.insert(key.clone());
            store.put(
                &key,
                serialize_entry_quantized(entry, QuantizationBits::Four).as_bytes(),
            )?;
        }

        for stale_key in store.keys_with_prefix("memory/") {
            if !live_keys.contains(&stale_key) {
                store.delete(&stale_key)?;
            }
        }

        store.put("meta/next_id", self.next_id.to_string().as_bytes())?;
        store.compact()
    }

    pub fn load_from_disk_kv(path: impl AsRef<Path>) -> io::Result<Self> {
        let store = DiskKvStore::open(path)?;
        load_cache_from_store(&store)
    }

    pub fn load_from_disk_kv_read_only_existing(
        path: impl AsRef<Path>,
    ) -> io::Result<Option<Self>> {
        let Some(store) = DiskKvStore::open_read_only_existing(path)? else {
            return Ok(None);
        };
        load_cache_from_store(&store).map(Some)
    }
}

fn load_cache_from_store(store: &DiskKvStore) -> io::Result<KvFusionCache> {
    let mut cache = KvFusionCache::new();
    let mut seen_ids = HashSet::new();

    for key in store.keys_with_prefix("memory/") {
        let Some(value) = store.get(&key)? else {
            continue;
        };
        let Ok(line) = String::from_utf8(value) else {
            continue;
        };
        let Some(entry) = deserialize_entry(&line) else {
            continue;
        };
        let key_id = key
            .strip_prefix("memory/")
            .and_then(|value| value.parse::<u64>().ok())
            .ok_or_else(|| {
                io::Error::new(
                    ErrorKind::InvalidData,
                    format!("invalid memory key in disk KV: {key}"),
                )
            })?;
        if entry.id != key_id {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                format!(
                    "memory key/id mismatch in disk KV: key={key_id} entry={}",
                    entry.id
                ),
            ));
        }
        if !seen_ids.insert(entry.id) {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                format!("duplicate memory id in disk KV: {}", entry.id),
            ));
        }
        cache.next_id = cache.next_id.max(entry.id + 1);
        cache.clock = cache.clock.max(entry.created_at).max(entry.last_access);
        cache.entries.push(entry);
    }

    if let Some(value) = store.get("meta/next_id")?
        && let Ok(next_id) = String::from_utf8_lossy(&value).parse::<u64>()
    {
        cache.next_id = cache.next_id.max(next_id);
    }
    cache.clock = cache.clock.saturating_add(1);

    Ok(cache)
}
