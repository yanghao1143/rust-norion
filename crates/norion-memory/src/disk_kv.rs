use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::str;

use crate::{
    MemoryAdapter, MemoryAdapterCapability, MemoryAdapterDescriptor, MemoryAdapterHealth,
    MemoryError, MemoryResult, stable_hash,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KvTier {
    Hot,
    Cold,
}

impl KvTier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hot => "hot",
            Self::Cold => "cold",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KvShardMetadata {
    pub id: String,
    pub byte_len: usize,
    pub checksum: u64,
    pub tier: KvTier,
    pub priority: f32,
    pub last_access: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColdKvShard {
    pub metadata: KvShardMetadata,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiskKvShardKeys {
    pub bytes_key: String,
    pub metadata_key: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiskKvShardManifest {
    pub keys: DiskKvShardKeys,
    pub metadata: KvShardMetadata,
}

impl DiskKvShardManifest {
    pub fn metadata_line(&self) -> String {
        serialize_kv_metadata(&self.metadata)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DiskKvCatalogVerification {
    pub manifests: Vec<DiskKvShardManifest>,
    pub missing_byte_ids: Vec<String>,
    pub byte_len_mismatch_ids: Vec<String>,
    pub checksum_mismatch_ids: Vec<String>,
}

impl DiskKvCatalogVerification {
    pub fn is_verified(&self) -> bool {
        self.catalog_verified() && self.checksum_verified()
    }

    pub fn catalog_verified(&self) -> bool {
        self.missing_byte_ids.is_empty() && self.byte_len_mismatch_ids.is_empty()
    }

    pub fn checksum_verified(&self) -> bool {
        self.checksum_mismatch_ids.is_empty()
    }

    pub fn record_count(&self) -> usize {
        self.manifests.len()
    }

    pub fn reason_codes(&self) -> Vec<String> {
        let mut codes = BTreeSet::new();
        if !self.byte_len_mismatch_ids.is_empty() {
            codes.insert("byte_len_mismatch".to_owned());
        }
        if !self.checksum_mismatch_ids.is_empty() {
            codes.insert("checksum_mismatch".to_owned());
        }
        if !self.missing_byte_ids.is_empty() {
            codes.insert("missing_bytes".to_owned());
        }
        codes.into_iter().collect()
    }

    pub fn detail_codes(&self) -> Vec<String> {
        self.missing_byte_ids
            .iter()
            .map(|id| format!("missing_bytes:{}", encode_hex(id.as_bytes())))
            .chain(
                self.byte_len_mismatch_ids
                    .iter()
                    .map(|id| format!("byte_len_mismatch:{}", encode_hex(id.as_bytes()))),
            )
            .chain(
                self.checksum_mismatch_ids
                    .iter()
                    .map(|id| format!("checksum_mismatch:{}", encode_hex(id.as_bytes()))),
            )
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "disk_kv_catalog verified={} catalog_verified={} checksum_verified={} records={} missing_bytes={} byte_len_mismatches={} checksum_mismatches={} missing_id_hex={} byte_len_mismatch_id_hex={} checksum_mismatch_id_hex={} reason_codes={} detail_codes={}",
            self.is_verified(),
            self.catalog_verified(),
            self.checksum_verified(),
            self.record_count(),
            self.missing_byte_ids.len(),
            self.byte_len_mismatch_ids.len(),
            self.checksum_mismatch_ids.len(),
            join_encoded_ids(&self.missing_byte_ids),
            join_encoded_ids(&self.byte_len_mismatch_ids),
            join_encoded_ids(&self.checksum_mismatch_ids),
            join_codes(&self.reason_codes()),
            join_codes(&self.detail_codes()),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiskKvShardKeyspace {
    pub prefix: String,
}

impl Default for DiskKvShardKeyspace {
    fn default() -> Self {
        Self {
            prefix: "kvswap/shard".to_owned(),
        }
    }
}

impl DiskKvShardKeyspace {
    pub fn new(prefix: impl Into<String>) -> Self {
        let mut prefix = prefix.into();
        while prefix.ends_with('/') {
            prefix.pop();
        }
        Self { prefix }
    }

    pub fn keys_for(&self, shard_id: &str) -> MemoryResult<DiskKvShardKeys> {
        validate_shard_id(shard_id)?;
        let encoded = encode_hex(shard_id.as_bytes());
        Ok(DiskKvShardKeys {
            bytes_key: format!("{}/{encoded}/bytes", self.prefix),
            metadata_key: format!("{}/{encoded}/metadata", self.prefix),
        })
    }

    pub fn catalog_prefix(&self) -> String {
        format!("{}/", self.prefix)
    }

    pub fn shard_id_from_key(&self, key: &str) -> Option<String> {
        let rest = key.strip_prefix(&self.catalog_prefix())?;
        let (encoded, suffix) = rest.split_once('/')?;
        matches!(suffix, "bytes" | "metadata")
            .then(|| decode_hex_to_string(encoded).ok())
            .flatten()
    }

    pub fn manifest_for(&self, metadata: KvShardMetadata) -> MemoryResult<DiskKvShardManifest> {
        let keys = self.keys_for(&metadata.id)?;
        Ok(DiskKvShardManifest { keys, metadata })
    }

    pub fn metadata_manifest_from_entry(
        &self,
        key: &str,
        value: &str,
    ) -> MemoryResult<Option<DiskKvShardManifest>> {
        if !key.starts_with(&self.catalog_prefix()) || !key.ends_with("/metadata") {
            return Ok(None);
        }
        let Some(id_from_key) = self.shard_id_from_key(key) else {
            return Err(MemoryError::InvalidInput(format!(
                "invalid disk kv metadata key {key}"
            )));
        };
        let metadata = deserialize_kv_metadata(value)?;
        if metadata.id != id_from_key {
            return Err(MemoryError::InvalidInput(format!(
                "metadata id {} does not match key id {}",
                metadata.id, id_from_key
            )));
        }
        Ok(Some(self.manifest_for(metadata)?))
    }

    pub fn catalog_manifests<'a, I>(&self, entries: I) -> MemoryResult<Vec<DiskKvShardManifest>>
    where
        I: IntoIterator<Item = (&'a str, &'a str)>,
    {
        let mut seen = BTreeSet::new();
        let mut manifests = Vec::new();
        for (key, value) in entries {
            let Some(manifest) = self.metadata_manifest_from_entry(key, value)? else {
                continue;
            };
            if !seen.insert(manifest.metadata.id.clone()) {
                return Err(MemoryError::InvalidInput(format!(
                    "duplicate disk kv metadata for shard {}",
                    manifest.metadata.id
                )));
            }
            manifests.push(manifest);
        }
        manifests.sort_by(|left, right| left.metadata.id.cmp(&right.metadata.id));
        Ok(manifests)
    }

    pub fn verify_catalog_entries<'a, I>(
        &self,
        entries: I,
    ) -> MemoryResult<DiskKvCatalogVerification>
    where
        I: IntoIterator<Item = (&'a str, &'a [u8])>,
    {
        let mut seen_metadata = BTreeSet::new();
        let mut seen_bytes = BTreeSet::new();
        let mut manifests = Vec::new();
        let mut bytes_by_id = BTreeMap::<String, &'a [u8]>::new();

        for (key, value) in entries {
            if key.starts_with(&self.catalog_prefix()) && key.ends_with("/metadata") {
                let value = str::from_utf8(value).map_err(|error| {
                    MemoryError::InvalidInput(format!(
                        "disk kv metadata value for {key} is not utf-8: {error}"
                    ))
                })?;
                let Some(manifest) = self.metadata_manifest_from_entry(key, value)? else {
                    continue;
                };
                if !seen_metadata.insert(manifest.metadata.id.clone()) {
                    return Err(MemoryError::InvalidInput(format!(
                        "duplicate disk kv metadata for shard {}",
                        manifest.metadata.id
                    )));
                }
                manifests.push(manifest);
            } else if key.starts_with(&self.catalog_prefix()) && key.ends_with("/bytes") {
                let Some(id) = self.shard_id_from_key(key) else {
                    return Err(MemoryError::InvalidInput(format!(
                        "invalid disk kv bytes key {key}"
                    )));
                };
                if !seen_bytes.insert(id.clone()) {
                    return Err(MemoryError::InvalidInput(format!(
                        "duplicate disk kv bytes for shard {id}"
                    )));
                }
                bytes_by_id.insert(id, value);
            }
        }

        manifests.sort_by(|left, right| left.metadata.id.cmp(&right.metadata.id));
        let mut missing_byte_ids = Vec::new();
        let mut byte_len_mismatch_ids = Vec::new();
        let mut checksum_mismatch_ids = Vec::new();

        for manifest in &manifests {
            let id = &manifest.metadata.id;
            let Some(bytes) = bytes_by_id.get(id.as_str()) else {
                missing_byte_ids.push(id.clone());
                continue;
            };
            if bytes.len() != manifest.metadata.byte_len {
                byte_len_mismatch_ids.push(id.clone());
            }
            if checksum(bytes) != manifest.metadata.checksum {
                checksum_mismatch_ids.push(id.clone());
            }
        }

        Ok(DiskKvCatalogVerification {
            manifests,
            missing_byte_ids,
            byte_len_mismatch_ids,
            checksum_mismatch_ids,
        })
    }
}

pub trait DiskKvOffload {
    fn write_cold_shard(&mut self, shard: ColdKvShard) -> MemoryResult<KvShardMetadata>;
    fn read_cold_shard(&mut self, id: &str) -> MemoryResult<Option<ColdKvShard>>;
    fn delete_cold_shard(&mut self, id: &str) -> MemoryResult<bool>;
    fn cold_metadata(&self, id: &str) -> Option<KvShardMetadata>;
    fn list_cold_metadata(&self) -> Vec<KvShardMetadata>;
}

pub fn serialize_kv_metadata(metadata: &KvShardMetadata) -> String {
    format!(
        "id_hex={} byte_len={} checksum={} tier={} priority={:.6} last_access={}",
        encode_hex(metadata.id.as_bytes()),
        metadata.byte_len,
        metadata.checksum,
        metadata.tier.as_str(),
        metadata.priority,
        metadata.last_access
    )
}

pub fn deserialize_kv_metadata(value: &str) -> MemoryResult<KvShardMetadata> {
    let fields = value
        .split_whitespace()
        .filter_map(|part| part.split_once('='))
        .collect::<BTreeMap<_, _>>();
    let id_hex = required_field(&fields, "id_hex")?;
    let tier = match required_field(&fields, "tier")? {
        "hot" => KvTier::Hot,
        "cold" => KvTier::Cold,
        other => {
            return Err(MemoryError::InvalidInput(format!(
                "unknown kv metadata tier {other}"
            )));
        }
    };
    Ok(KvShardMetadata {
        id: decode_hex_to_string(id_hex)?,
        byte_len: parse_field(&fields, "byte_len")?,
        checksum: parse_field(&fields, "checksum")?,
        tier,
        priority: parse_field::<f32>(&fields, "priority")?.clamp(0.0, 1.0),
        last_access: parse_field(&fields, "last_access")?,
    })
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryDiskKvOffload {
    shards: BTreeMap<String, ColdKvShard>,
}

impl InMemoryDiskKvOffload {
    pub fn new() -> Self {
        Self::default()
    }
}

impl MemoryAdapter for InMemoryDiskKvOffload {
    fn descriptor(&self) -> MemoryAdapterDescriptor {
        MemoryAdapterDescriptor::new(
            "in_memory_disk_kv_offload",
            vec![MemoryAdapterCapability::DiskKvOffload],
        )
    }

    fn health(&self) -> MemoryResult<MemoryAdapterHealth> {
        Ok(MemoryAdapterHealth::ready(Some(self.shards.len())))
    }
}

impl DiskKvOffload for InMemoryDiskKvOffload {
    fn write_cold_shard(&mut self, mut shard: ColdKvShard) -> MemoryResult<KvShardMetadata> {
        validate_shard_id(&shard.metadata.id)?;
        shard.metadata.tier = KvTier::Cold;
        shard.metadata.byte_len = shard.bytes.len();
        shard.metadata.checksum = checksum(&shard.bytes);
        let metadata = shard.metadata.clone();
        self.shards.insert(metadata.id.clone(), shard);
        Ok(metadata)
    }

    fn read_cold_shard(&mut self, id: &str) -> MemoryResult<Option<ColdKvShard>> {
        Ok(self.shards.get(id).cloned())
    }

    fn delete_cold_shard(&mut self, id: &str) -> MemoryResult<bool> {
        Ok(self.shards.remove(id).is_some())
    }

    fn cold_metadata(&self, id: &str) -> Option<KvShardMetadata> {
        self.shards.get(id).map(|shard| shard.metadata.clone())
    }

    fn list_cold_metadata(&self) -> Vec<KvShardMetadata> {
        self.shards
            .values()
            .map(|shard| shard.metadata.clone())
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct FileDiskKvOffload {
    root: PathBuf,
    metadata: BTreeMap<String, KvShardMetadata>,
}

impl FileDiskKvOffload {
    pub fn open(root: impl AsRef<Path>) -> MemoryResult<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        Ok(Self {
            root,
            metadata: BTreeMap::new(),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn shard_path(&self, id: &str) -> PathBuf {
        self.root
            .join(format!("{:016x}.kvshard", stable_hash(id.as_bytes())))
    }
}

impl MemoryAdapter for FileDiskKvOffload {
    fn descriptor(&self) -> MemoryAdapterDescriptor {
        MemoryAdapterDescriptor::new(
            "file_disk_kv_offload",
            vec![MemoryAdapterCapability::DiskKvOffload],
        )
    }

    fn health(&self) -> MemoryResult<MemoryAdapterHealth> {
        Ok(MemoryAdapterHealth::ready(Some(self.metadata.len())))
    }
}

impl DiskKvOffload for FileDiskKvOffload {
    fn write_cold_shard(&mut self, mut shard: ColdKvShard) -> MemoryResult<KvShardMetadata> {
        validate_shard_id(&shard.metadata.id)?;
        shard.metadata.tier = KvTier::Cold;
        shard.metadata.byte_len = shard.bytes.len();
        shard.metadata.checksum = checksum(&shard.bytes);
        fs::write(self.shard_path(&shard.metadata.id), &shard.bytes)?;
        let metadata = shard.metadata.clone();
        self.metadata.insert(metadata.id.clone(), metadata.clone());
        Ok(metadata)
    }

    fn read_cold_shard(&mut self, id: &str) -> MemoryResult<Option<ColdKvShard>> {
        let Some(metadata) = self.metadata.get(id).cloned() else {
            return Ok(None);
        };
        let bytes = fs::read(self.shard_path(id))?;
        if checksum(&bytes) != metadata.checksum {
            return Err(MemoryError::InvalidInput(format!(
                "cold shard checksum mismatch for {id}"
            )));
        }
        Ok(Some(ColdKvShard { metadata, bytes }))
    }

    fn delete_cold_shard(&mut self, id: &str) -> MemoryResult<bool> {
        let Some(_) = self.metadata.remove(id) else {
            return Ok(false);
        };
        let path = self.shard_path(id);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(true)
    }

    fn cold_metadata(&self, id: &str) -> Option<KvShardMetadata> {
        self.metadata.get(id).cloned()
    }

    fn list_cold_metadata(&self) -> Vec<KvShardMetadata> {
        self.metadata.values().cloned().collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KvEvictionPlan {
    pub demote_ids: Vec<String>,
    pub keep_hot_ids: Vec<String>,
    pub target_hot_bytes: usize,
    pub reason: String,
}

impl KvEvictionPlan {
    pub fn demote_count(&self) -> usize {
        self.demote_ids.len()
    }

    pub fn keep_hot_count(&self) -> usize {
        self.keep_hot_ids.len()
    }

    pub fn reason_codes(&self) -> Vec<String> {
        let mut codes = BTreeSet::new();
        if !self.reason.trim().is_empty() {
            codes.insert(normalized_code(&self.reason));
        }
        if !self.demote_ids.is_empty() {
            codes.insert("evict_demote".to_owned());
        }
        if !self.keep_hot_ids.is_empty() {
            codes.insert("evict_keep_hot".to_owned());
        }
        codes.into_iter().collect()
    }

    pub fn detail_codes(&self) -> Vec<String> {
        let reason = plan_reason_code(&self.reason);
        self.demote_ids
            .iter()
            .map(|id| format!("demote:{reason}:{}", encode_hex(id.as_bytes())))
            .chain(
                self.keep_hot_ids
                    .iter()
                    .map(|id| format!("keep_hot:{}", encode_hex(id.as_bytes()))),
            )
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn detail_codes_for_action(&self, action: &str) -> Vec<String> {
        let prefix = format!("{}:", normalized_code(action));
        self.detail_codes()
            .into_iter()
            .filter(|code| code.starts_with(&prefix))
            .collect()
    }

    pub fn detail_codes_for_reason(&self, reason: &str) -> Vec<String> {
        let needle = format!(":{}:", plan_reason_code(reason));
        self.detail_codes()
            .into_iter()
            .filter(|code| code.contains(&needle))
            .collect()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "kvswap_eviction target_hot_bytes={} demote={} keep_hot={} reason={} demote_id_hex={} keep_hot_id_hex={} reason_codes={} detail_codes={}",
            self.target_hot_bytes,
            self.demote_count(),
            self.keep_hot_count(),
            self.reason,
            join_encoded_ids(&self.demote_ids),
            join_encoded_ids(&self.keep_hot_ids),
            join_codes(&self.reason_codes()),
            join_codes(&self.detail_codes()),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KvPrefetchPlan {
    pub promote_ids: Vec<String>,
    pub missing_ids: Vec<String>,
    pub already_hot_ids: Vec<String>,
    pub duplicate_ids: Vec<String>,
    pub reason: String,
}

impl KvPrefetchPlan {
    pub fn promote_count(&self) -> usize {
        self.promote_ids.len()
    }

    pub fn missing_count(&self) -> usize {
        self.missing_ids.len()
    }

    pub fn already_hot_count(&self) -> usize {
        self.already_hot_ids.len()
    }

    pub fn duplicate_count(&self) -> usize {
        self.duplicate_ids.len()
    }

    pub fn reason_codes(&self) -> Vec<String> {
        let mut codes = BTreeSet::new();
        if !self.reason.trim().is_empty() {
            codes.insert(normalized_code(&self.reason));
        }
        if !self.promote_ids.is_empty() {
            codes.insert("prefetch_promote".to_owned());
        }
        if !self.missing_ids.is_empty() {
            codes.insert("prefetch_missing".to_owned());
        }
        if !self.already_hot_ids.is_empty() {
            codes.insert("prefetch_already_hot".to_owned());
        }
        if !self.duplicate_ids.is_empty() {
            codes.insert("prefetch_duplicate".to_owned());
        }
        codes.into_iter().collect()
    }

    pub fn detail_codes(&self) -> Vec<String> {
        let reason = plan_reason_code(&self.reason);
        self.promote_ids
            .iter()
            .map(|id| format!("promote:{reason}:{}", encode_hex(id.as_bytes())))
            .chain(
                self.missing_ids
                    .iter()
                    .map(|id| format!("missing:{reason}:{}", encode_hex(id.as_bytes()))),
            )
            .chain(
                self.already_hot_ids
                    .iter()
                    .map(|id| format!("already_hot:{}", encode_hex(id.as_bytes()))),
            )
            .chain(
                self.duplicate_ids
                    .iter()
                    .map(|id| format!("duplicate:{}", encode_hex(id.as_bytes()))),
            )
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn detail_codes_for_action(&self, action: &str) -> Vec<String> {
        let prefix = format!("{}:", normalized_code(action));
        self.detail_codes()
            .into_iter()
            .filter(|code| code.starts_with(&prefix))
            .collect()
    }

    pub fn detail_codes_for_reason(&self, reason: &str) -> Vec<String> {
        let needle = format!(":{}:", plan_reason_code(reason));
        self.detail_codes()
            .into_iter()
            .filter(|code| code.contains(&needle))
            .collect()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "kvswap_prefetch promote={} missing={} hot={} duplicate={} reason={} promote_id_hex={} missing_id_hex={} hot_id_hex={} duplicate_id_hex={} reason_codes={} detail_codes={}",
            self.promote_count(),
            self.missing_count(),
            self.already_hot_count(),
            self.duplicate_count(),
            self.reason,
            join_encoded_ids(&self.promote_ids),
            join_encoded_ids(&self.missing_ids),
            join_encoded_ids(&self.already_hot_ids),
            join_encoded_ids(&self.duplicate_ids),
            join_codes(&self.reason_codes()),
            join_codes(&self.detail_codes()),
        )
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KvSwapStateSnapshot {
    pub hot_shard_count: usize,
    pub cold_shard_count: usize,
    pub metadata_count: usize,
    pub hot_byte_len: usize,
    pub cold_byte_len: usize,
}

impl KvSwapStateSnapshot {
    pub fn is_empty(&self) -> bool {
        self.hot_shard_count == 0
            && self.cold_shard_count == 0
            && self.metadata_count == 0
            && self.hot_byte_len == 0
            && self.cold_byte_len == 0
    }

    pub fn total_byte_len(&self) -> usize {
        self.hot_byte_len.saturating_add(self.cold_byte_len)
    }

    pub fn shape_codes(&self) -> Vec<String> {
        if self.is_empty() {
            return vec!["empty".to_owned()];
        }

        let mut codes = BTreeSet::new();
        if self.metadata_count > 0 {
            codes.insert("metadata_index".to_owned());
        }
        if self.hot_shard_count > 0 {
            codes.insert("hot_metadata".to_owned());
        }
        if self.cold_shard_count > 0 {
            codes.insert("cold_catalog".to_owned());
        }
        match (self.hot_shard_count > 0, self.cold_shard_count > 0) {
            (true, true) => {
                codes.insert("mixed_tiers".to_owned());
            }
            (true, false) => {
                codes.insert("hot_only".to_owned());
            }
            (false, true) => {
                codes.insert("cold_only".to_owned());
            }
            (false, false) => {
                codes.insert("metadata_only".to_owned());
            }
        }
        codes.into_iter().collect()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "kvswap_state empty={} hot={} cold={} metadata={} hot_bytes={} cold_bytes={} total_bytes={} shape_codes={}",
            self.is_empty(),
            self.hot_shard_count,
            self.cold_shard_count,
            self.metadata_count,
            self.hot_byte_len,
            self.cold_byte_len,
            self.total_byte_len(),
            join_codes(&self.shape_codes()),
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KvSwapBoundaryAudit {
    pub overlapping_hot_cold_ids: Vec<String>,
    pub missing_hot_metadata_ids: Vec<String>,
    pub stale_metadata_ids: Vec<String>,
    pub hot_tier_mismatch_ids: Vec<String>,
    pub cold_tier_mismatch_ids: Vec<String>,
}

impl KvSwapBoundaryAudit {
    pub fn is_clean(&self) -> bool {
        self.overlapping_hot_cold_ids.is_empty()
            && self.missing_hot_metadata_ids.is_empty()
            && self.stale_metadata_ids.is_empty()
            && self.hot_tier_mismatch_ids.is_empty()
            && self.cold_tier_mismatch_ids.is_empty()
    }

    pub fn issue_count(&self) -> usize {
        self.overlapping_hot_cold_ids.len()
            + self.missing_hot_metadata_ids.len()
            + self.stale_metadata_ids.len()
            + self.hot_tier_mismatch_ids.len()
            + self.cold_tier_mismatch_ids.len()
    }

    pub fn readiness(&self) -> KvSwapBoundaryReadiness {
        KvSwapBoundaryReadiness::from_audit(self)
    }

    pub fn reason_codes(&self) -> Vec<String> {
        let mut codes = BTreeSet::new();
        if !self.overlapping_hot_cold_ids.is_empty() {
            codes.insert("overlapping_hot_cold".to_owned());
        }
        if !self.missing_hot_metadata_ids.is_empty() {
            codes.insert("missing_hot_metadata".to_owned());
        }
        if !self.stale_metadata_ids.is_empty() {
            codes.insert("stale_metadata".to_owned());
        }
        if !self.hot_tier_mismatch_ids.is_empty() {
            codes.insert("hot_tier_mismatch".to_owned());
        }
        if !self.cold_tier_mismatch_ids.is_empty() {
            codes.insert("cold_tier_mismatch".to_owned());
        }
        codes.into_iter().collect()
    }

    pub fn detail_codes(&self) -> Vec<String> {
        self.overlapping_hot_cold_ids
            .iter()
            .map(|id| format!("overlap:{}", encode_hex(id.as_bytes())))
            .chain(
                self.missing_hot_metadata_ids
                    .iter()
                    .map(|id| format!("missing_hot_metadata:{}", encode_hex(id.as_bytes()))),
            )
            .chain(
                self.stale_metadata_ids
                    .iter()
                    .map(|id| format!("stale_metadata:{}", encode_hex(id.as_bytes()))),
            )
            .chain(
                self.hot_tier_mismatch_ids
                    .iter()
                    .map(|id| format!("hot_tier_mismatch:{}", encode_hex(id.as_bytes()))),
            )
            .chain(
                self.cold_tier_mismatch_ids
                    .iter()
                    .map(|id| format!("cold_tier_mismatch:{}", encode_hex(id.as_bytes()))),
            )
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn boundary_blocker_detail_codes(&self) -> Vec<String> {
        self.overlapping_hot_cold_ids
            .iter()
            .map(|id| format!("overlap:{}", encode_hex(id.as_bytes())))
            .chain(
                self.missing_hot_metadata_ids
                    .iter()
                    .map(|id| format!("missing_hot_metadata:{}", encode_hex(id.as_bytes()))),
            )
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn boundary_warning_detail_codes(&self) -> Vec<String> {
        self.stale_metadata_ids
            .iter()
            .map(|id| format!("stale_metadata:{}", encode_hex(id.as_bytes())))
            .chain(
                self.hot_tier_mismatch_ids
                    .iter()
                    .map(|id| format!("hot_tier_mismatch:{}", encode_hex(id.as_bytes()))),
            )
            .chain(
                self.cold_tier_mismatch_ids
                    .iter()
                    .map(|id| format!("cold_tier_mismatch:{}", encode_hex(id.as_bytes()))),
            )
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "kvswap_boundary clean={} issues={} overlap={} missing_hot_metadata={} stale_metadata={} hot_tier_mismatch={} cold_tier_mismatch={} reason_codes={} detail_codes={}",
            self.is_clean(),
            self.issue_count(),
            self.overlapping_hot_cold_ids.len(),
            self.missing_hot_metadata_ids.len(),
            self.stale_metadata_ids.len(),
            self.hot_tier_mismatch_ids.len(),
            self.cold_tier_mismatch_ids.len(),
            join_codes(&self.reason_codes()),
            join_codes(&self.detail_codes()),
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KvSwapBoundaryReadiness {
    pub ready_for_kvswap: bool,
    pub blocker_count: usize,
    pub warning_count: usize,
    pub blocker_reason_codes: Vec<String>,
    pub warning_reason_codes: Vec<String>,
    pub blocker_detail_codes: Vec<String>,
    pub warning_detail_codes: Vec<String>,
}

impl KvSwapBoundaryReadiness {
    pub fn from_audit(audit: &KvSwapBoundaryAudit) -> Self {
        let blocker_count =
            audit.overlapping_hot_cold_ids.len() + audit.missing_hot_metadata_ids.len();
        let warning_count = audit.stale_metadata_ids.len()
            + audit.hot_tier_mismatch_ids.len()
            + audit.cold_tier_mismatch_ids.len();
        let mut blocker_reason_codes = Vec::new();
        let mut warning_reason_codes = Vec::new();

        if !audit.overlapping_hot_cold_ids.is_empty() {
            blocker_reason_codes.push("overlapping_hot_cold".to_owned());
        }
        if !audit.missing_hot_metadata_ids.is_empty() {
            blocker_reason_codes.push("missing_hot_metadata".to_owned());
        }
        if !audit.stale_metadata_ids.is_empty() {
            warning_reason_codes.push("stale_metadata".to_owned());
        }
        if !audit.hot_tier_mismatch_ids.is_empty() {
            warning_reason_codes.push("hot_tier_mismatch".to_owned());
        }
        if !audit.cold_tier_mismatch_ids.is_empty() {
            warning_reason_codes.push("cold_tier_mismatch".to_owned());
        }
        blocker_reason_codes.sort();
        warning_reason_codes.sort();

        Self {
            ready_for_kvswap: blocker_count == 0,
            blocker_count,
            warning_count,
            blocker_reason_codes,
            warning_reason_codes,
            blocker_detail_codes: audit.boundary_blocker_detail_codes(),
            warning_detail_codes: audit.boundary_warning_detail_codes(),
        }
    }

    pub fn reason_codes(&self) -> Vec<String> {
        self.blocker_reason_codes
            .iter()
            .chain(self.warning_reason_codes.iter())
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn detail_codes(&self) -> Vec<String> {
        self.blocker_detail_codes
            .iter()
            .map(|code| format!("blocker:{code}"))
            .chain(
                self.warning_detail_codes
                    .iter()
                    .map(|code| format!("warning:{code}")),
            )
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "kvswap_boundary_readiness ready={} blockers={} warnings={} blocker_reason_codes={} warning_reason_codes={} detail_codes={}",
            self.ready_for_kvswap,
            self.blocker_count,
            self.warning_count,
            join_codes(&self.blocker_reason_codes),
            join_codes(&self.warning_reason_codes),
            join_codes(&self.detail_codes()),
        )
    }
}

pub trait KvSwap {
    fn stage_hot(&mut self, id: String, bytes: Vec<u8>, priority: f32) -> MemoryResult<()>;
    fn plan_eviction(&self, target_hot_bytes: usize) -> KvEvictionPlan;
    fn evict(&mut self, plan: &KvEvictionPlan) -> MemoryResult<Vec<KvShardMetadata>>;
    fn plan_prefetch(&self, ids: &[String]) -> KvPrefetchPlan;
    fn prefetch(&mut self, plan: &KvPrefetchPlan) -> MemoryResult<Vec<String>>;
    fn metadata(&self, id: &str) -> Option<KvShardMetadata>;
    fn hot_bytes(&self, id: &str) -> Option<&[u8]>;
    fn hot_byte_len(&self) -> usize;
}

#[derive(Debug, Clone)]
pub struct KvSwapManager<B> {
    backend: B,
    hot: BTreeMap<String, Vec<u8>>,
    metadata: BTreeMap<String, KvShardMetadata>,
    clock: u64,
}

impl<B: DiskKvOffload> KvSwapManager<B> {
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            hot: BTreeMap::new(),
            metadata: BTreeMap::new(),
            clock: 0,
        }
    }

    pub fn backend(&self) -> &B {
        &self.backend
    }

    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }

    pub fn state_snapshot(&self) -> KvSwapStateSnapshot {
        let cold_metadata = self.backend.list_cold_metadata();
        let mut metadata_ids = self.metadata.keys().cloned().collect::<BTreeSet<_>>();
        metadata_ids.extend(cold_metadata.iter().map(|metadata| metadata.id.clone()));
        KvSwapStateSnapshot {
            hot_shard_count: self.hot.len(),
            cold_shard_count: cold_metadata.len(),
            metadata_count: metadata_ids.len(),
            hot_byte_len: self.hot_byte_len(),
            cold_byte_len: cold_metadata.iter().map(|metadata| metadata.byte_len).sum(),
        }
    }

    pub fn boundary_audit(&self) -> KvSwapBoundaryAudit {
        let hot_ids = self.hot.keys().cloned().collect::<BTreeSet<_>>();
        let cold_metadata = self.backend.list_cold_metadata();
        let cold_ids = cold_metadata
            .iter()
            .map(|metadata| metadata.id.clone())
            .collect::<BTreeSet<_>>();
        let live_ids = hot_ids.union(&cold_ids).cloned().collect::<BTreeSet<_>>();

        let overlapping_hot_cold_ids = hot_ids.intersection(&cold_ids).cloned().collect();
        let missing_hot_metadata_ids = hot_ids
            .iter()
            .filter(|id| !self.metadata.contains_key(*id))
            .cloned()
            .collect();
        let stale_metadata_ids = self
            .metadata
            .keys()
            .filter(|id| !live_ids.contains(*id))
            .cloned()
            .collect();
        let hot_tier_mismatch_ids = hot_ids
            .iter()
            .filter(|id| {
                self.metadata
                    .get(*id)
                    .is_some_and(|metadata| metadata.tier != KvTier::Hot)
            })
            .cloned()
            .collect();
        let cold_tier_mismatch_ids = cold_metadata
            .iter()
            .filter(|metadata| metadata.tier != KvTier::Cold)
            .map(|metadata| metadata.id.clone())
            .collect();

        KvSwapBoundaryAudit {
            overlapping_hot_cold_ids,
            missing_hot_metadata_ids,
            stale_metadata_ids,
            hot_tier_mismatch_ids,
            cold_tier_mismatch_ids,
        }
    }
}

impl<B: DiskKvOffload> MemoryAdapter for KvSwapManager<B> {
    fn descriptor(&self) -> MemoryAdapterDescriptor {
        MemoryAdapterDescriptor::new(
            "kvswap_manager",
            vec![
                MemoryAdapterCapability::KvSwap,
                MemoryAdapterCapability::DiskKvOffload,
            ],
        )
    }

    fn health(&self) -> MemoryResult<MemoryAdapterHealth> {
        let mut ids = self.metadata.keys().cloned().collect::<BTreeSet<_>>();
        ids.extend(
            self.backend
                .list_cold_metadata()
                .into_iter()
                .map(|metadata| metadata.id),
        );
        Ok(MemoryAdapterHealth::ready(Some(ids.len())))
    }
}

impl<B: DiskKvOffload> KvSwap for KvSwapManager<B> {
    fn stage_hot(&mut self, id: String, bytes: Vec<u8>, priority: f32) -> MemoryResult<()> {
        validate_shard_id(&id)?;
        self.backend.delete_cold_shard(&id)?;
        self.clock = self.clock.saturating_add(1);
        let metadata = KvShardMetadata {
            id: id.clone(),
            byte_len: bytes.len(),
            checksum: checksum(&bytes),
            tier: KvTier::Hot,
            priority: priority.clamp(0.0, 1.0),
            last_access: self.clock,
        };
        self.hot.insert(id.clone(), bytes);
        self.metadata.insert(id, metadata);
        Ok(())
    }

    fn plan_eviction(&self, target_hot_bytes: usize) -> KvEvictionPlan {
        let mut current_bytes = self.hot_byte_len();
        let mut candidates = self
            .hot
            .keys()
            .filter_map(|id| self.metadata.get(id))
            .cloned()
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            left.priority
                .partial_cmp(&right.priority)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.last_access.cmp(&right.last_access))
        });

        let mut demote_ids = Vec::new();
        for metadata in candidates {
            if current_bytes <= target_hot_bytes {
                break;
            }
            current_bytes = current_bytes.saturating_sub(metadata.byte_len);
            demote_ids.push(metadata.id);
        }

        let keep_hot_ids = self
            .hot
            .keys()
            .filter(|id| !demote_ids.iter().any(|demote| demote == *id))
            .cloned()
            .collect::<Vec<_>>();
        KvEvictionPlan {
            demote_ids,
            keep_hot_ids,
            target_hot_bytes,
            reason: "target_hot_bytes".to_owned(),
        }
    }

    fn evict(&mut self, plan: &KvEvictionPlan) -> MemoryResult<Vec<KvShardMetadata>> {
        let mut demoted = Vec::new();
        for id in &plan.demote_ids {
            let Some(bytes) = self.hot.remove(id) else {
                continue;
            };
            let mut metadata = self.metadata.get(id).cloned().ok_or_else(|| {
                MemoryError::NotFound(format!("hot metadata missing for shard {id}"))
            })?;
            metadata.tier = KvTier::Cold;
            metadata.byte_len = bytes.len();
            metadata.checksum = checksum(&bytes);
            let cold_metadata = self.backend.write_cold_shard(ColdKvShard {
                metadata: metadata.clone(),
                bytes,
            })?;
            self.metadata.insert(id.clone(), cold_metadata.clone());
            demoted.push(cold_metadata);
        }
        Ok(demoted)
    }

    fn plan_prefetch(&self, ids: &[String]) -> KvPrefetchPlan {
        let mut promote_ids = Vec::new();
        let mut missing_ids = Vec::new();
        let mut already_hot_ids = Vec::new();
        let mut duplicate_ids = Vec::new();
        let mut seen = BTreeSet::new();
        for id in ids {
            if !seen.insert(id.clone()) {
                duplicate_ids.push(id.clone());
                continue;
            }
            match self.metadata(id).map(|metadata| metadata.tier) {
                Some(KvTier::Cold) => promote_ids.push(id.clone()),
                Some(KvTier::Hot) => already_hot_ids.push(id.clone()),
                None => missing_ids.push(id.clone()),
            }
        }
        KvPrefetchPlan {
            promote_ids,
            missing_ids,
            already_hot_ids,
            duplicate_ids,
            reason: "requested_ids".to_owned(),
        }
    }

    fn prefetch(&mut self, plan: &KvPrefetchPlan) -> MemoryResult<Vec<String>> {
        let mut promoted = Vec::new();
        let mut seen = BTreeSet::new();
        for id in &plan.promote_ids {
            if !seen.insert(id.clone()) {
                continue;
            }
            let Some(shard) = self.backend.read_cold_shard(id)? else {
                continue;
            };
            let mut metadata = shard.metadata.clone();
            self.clock = self.clock.max(metadata.last_access).saturating_add(1);
            metadata.tier = KvTier::Hot;
            metadata.byte_len = shard.bytes.len();
            metadata.checksum = checksum(&shard.bytes);
            metadata.last_access = self.clock;
            self.hot.insert(id.clone(), shard.bytes);
            self.metadata.insert(id.clone(), metadata);
            self.backend.delete_cold_shard(id)?;
            promoted.push(id.clone());
        }
        Ok(promoted)
    }

    fn metadata(&self, id: &str) -> Option<KvShardMetadata> {
        self.metadata
            .get(id)
            .cloned()
            .or_else(|| self.backend.cold_metadata(id))
    }

    fn hot_bytes(&self, id: &str) -> Option<&[u8]> {
        self.hot.get(id).map(Vec::as_slice)
    }

    fn hot_byte_len(&self) -> usize {
        self.hot.values().map(Vec::len).sum()
    }
}

fn validate_shard_id(id: &str) -> MemoryResult<()> {
    if id.trim().is_empty() {
        return Err(MemoryError::InvalidInput(
            "kv shard id cannot be empty".to_owned(),
        ));
    }
    Ok(())
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn join_encoded_ids(ids: &[String]) -> String {
    if ids.is_empty() {
        return "none".to_owned();
    }
    ids.iter()
        .map(|id| encode_hex(id.as_bytes()))
        .collect::<Vec<_>>()
        .join("|")
}

fn plan_reason_code(reason: &str) -> String {
    if reason.trim().is_empty() {
        "unspecified".to_owned()
    } else {
        normalized_code(reason)
    }
}

fn join_codes(codes: &[String]) -> String {
    if codes.is_empty() {
        return "none".to_owned();
    }
    codes.join("|")
}

fn normalized_code(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == ':' {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_owned()
}

fn decode_hex_to_string(value: &str) -> MemoryResult<String> {
    if !value.len().is_multiple_of(2) {
        return Err(MemoryError::InvalidInput(
            "hex encoded shard id must have even length".to_owned(),
        ));
    }
    let mut bytes = Vec::with_capacity(value.len() / 2);
    let mut chars = value.as_bytes().chunks_exact(2);
    for pair in &mut chars {
        let text = std::str::from_utf8(pair).map_err(|error| {
            MemoryError::InvalidInput(format!("invalid hex shard id bytes: {error}"))
        })?;
        let byte = u8::from_str_radix(text, 16).map_err(|error| {
            MemoryError::InvalidInput(format!("invalid hex shard id {value}: {error}"))
        })?;
        bytes.push(byte);
    }
    String::from_utf8(bytes).map_err(|error| {
        MemoryError::InvalidInput(format!("decoded shard id is not utf-8: {error}"))
    })
}

fn required_field<'a>(fields: &'a BTreeMap<&str, &str>, field: &str) -> MemoryResult<&'a str> {
    fields
        .get(field)
        .copied()
        .ok_or_else(|| MemoryError::InvalidInput(format!("missing kv metadata field {field}")))
}

fn parse_field<T>(fields: &BTreeMap<&str, &str>, field: &str) -> MemoryResult<T>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let value = required_field(fields, field)?;
    value.parse::<T>().map_err(|error| {
        MemoryError::InvalidInput(format!(
            "invalid kv metadata field {field}={value}: {error}"
        ))
    })
}

fn checksum(bytes: &[u8]) -> u64 {
    stable_hash(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kvswap_moves_hot_to_cold_and_prefetches_back() {
        let backend = InMemoryDiskKvOffload::new();
        let mut swap = KvSwapManager::new(backend);
        swap.stage_hot("a".to_owned(), vec![1, 2, 3, 4], 0.1)
            .unwrap();
        swap.stage_hot("b".to_owned(), vec![5, 6, 7, 8], 0.9)
            .unwrap();

        let plan = swap.plan_eviction(4);
        assert_eq!(plan.demote_ids, vec!["a".to_owned()]);
        let demoted = swap.evict(&plan).unwrap();
        assert_eq!(demoted[0].tier, KvTier::Cold);
        assert!(swap.hot_bytes("a").is_none());
        assert_eq!(swap.metadata("a").unwrap().tier, KvTier::Cold);

        let prefetch = swap.plan_prefetch(&["a".to_owned(), "missing".to_owned()]);
        assert_eq!(prefetch.promote_ids, vec!["a".to_owned()]);
        assert_eq!(prefetch.missing_ids, vec!["missing".to_owned()]);
        assert_eq!(swap.prefetch(&prefetch).unwrap(), vec!["a".to_owned()]);
        assert_eq!(swap.hot_bytes("a"), Some([1, 2, 3, 4].as_slice()));
        assert!(swap.boundary_audit().is_clean());
    }

    #[test]
    fn kvswap_boundary_audit_flags_hot_cold_overlap_and_metadata_drift() {
        let backend = InMemoryDiskKvOffload::new();
        let mut swap = KvSwapManager::new(backend);
        swap.stage_hot("overlap".to_owned(), vec![1, 2, 3], 0.7)
            .unwrap();
        swap.stage_hot("missing-meta".to_owned(), vec![4, 5], 0.6)
            .unwrap();
        swap.metadata.remove("missing-meta");
        swap.metadata.insert(
            "stale".to_owned(),
            KvShardMetadata {
                id: "stale".to_owned(),
                byte_len: 1,
                checksum: 1,
                tier: KvTier::Cold,
                priority: 0.1,
                last_access: 1,
            },
        );
        swap.backend_mut()
            .write_cold_shard(ColdKvShard {
                metadata: KvShardMetadata {
                    id: "overlap".to_owned(),
                    byte_len: 3,
                    checksum: checksum(&[1, 2, 3]),
                    tier: KvTier::Cold,
                    priority: 0.7,
                    last_access: 2,
                },
                bytes: vec![1, 2, 3],
            })
            .unwrap();

        let audit = swap.boundary_audit();

        assert!(!audit.is_clean());
        assert_eq!(audit.issue_count(), 3);
        assert_eq!(audit.overlapping_hot_cold_ids, vec!["overlap"]);
        assert_eq!(audit.missing_hot_metadata_ids, vec!["missing-meta"]);
        assert_eq!(audit.stale_metadata_ids, vec!["stale"]);
        assert_eq!(
            audit.reason_codes(),
            vec![
                "missing_hot_metadata".to_owned(),
                "overlapping_hot_cold".to_owned(),
                "stale_metadata".to_owned(),
            ]
        );
        assert_eq!(
            audit.detail_codes(),
            vec![
                "missing_hot_metadata:6d697373696e672d6d657461".to_owned(),
                "overlap:6f7665726c6170".to_owned(),
                "stale_metadata:7374616c65".to_owned(),
            ]
        );
        assert_eq!(
            audit.summary_line(),
            "kvswap_boundary clean=false issues=3 overlap=1 missing_hot_metadata=1 stale_metadata=1 hot_tier_mismatch=0 cold_tier_mismatch=0 reason_codes=missing_hot_metadata|overlapping_hot_cold|stale_metadata detail_codes=missing_hot_metadata:6d697373696e672d6d657461|overlap:6f7665726c6170|stale_metadata:7374616c65"
        );
        let readiness = audit.readiness();
        assert!(!readiness.ready_for_kvswap);
        assert_eq!(readiness.blocker_count, 2);
        assert_eq!(readiness.warning_count, 1);
        assert_eq!(
            readiness.blocker_reason_codes,
            vec![
                "missing_hot_metadata".to_owned(),
                "overlapping_hot_cold".to_owned()
            ]
        );
        assert_eq!(
            readiness.warning_reason_codes,
            vec!["stale_metadata".to_owned()]
        );
        assert_eq!(
            readiness.detail_codes(),
            vec![
                "blocker:missing_hot_metadata:6d697373696e672d6d657461".to_owned(),
                "blocker:overlap:6f7665726c6170".to_owned(),
                "warning:stale_metadata:7374616c65".to_owned(),
            ]
        );
        assert_eq!(
            readiness.summary_line(),
            "kvswap_boundary_readiness ready=false blockers=2 warnings=1 blocker_reason_codes=missing_hot_metadata|overlapping_hot_cold warning_reason_codes=stale_metadata detail_codes=blocker:missing_hot_metadata:6d697373696e672d6d657461|blocker:overlap:6f7665726c6170|warning:stale_metadata:7374616c65"
        );
    }

    #[test]
    fn disk_kv_keyspace_encodes_shard_ids_for_append_only_store() {
        let keyspace = DiskKvShardKeyspace::default();
        let keys = keyspace.keys_for("task/kv shard").unwrap();

        assert_eq!(
            keys.bytes_key,
            "kvswap/shard/7461736b2f6b76207368617264/bytes"
        );
        assert_eq!(
            keys.metadata_key,
            "kvswap/shard/7461736b2f6b76207368617264/metadata"
        );
        assert_eq!(keyspace.catalog_prefix(), "kvswap/shard/");
        assert_eq!(
            keyspace.shard_id_from_key(&keys.metadata_key).as_deref(),
            Some("task/kv shard")
        );
        assert_eq!(keyspace.shard_id_from_key("kvswap/shard/bad/unknown"), None);
    }

    #[test]
    fn kv_metadata_serializes_for_disk_kv_adapter_round_trip() {
        let metadata = KvShardMetadata {
            id: "runtime shard/1".to_owned(),
            byte_len: 12,
            checksum: 99,
            tier: KvTier::Cold,
            priority: 0.4567894,
            last_access: 77,
        };

        let serialized = serialize_kv_metadata(&metadata);
        assert!(serialized.contains("id_hex=72756e74696d652073686172642f31"));
        let parsed = deserialize_kv_metadata(&serialized).unwrap();
        assert_eq!(parsed.id, metadata.id);
        assert_eq!(parsed.byte_len, metadata.byte_len);
        assert_eq!(parsed.checksum, metadata.checksum);
        assert_eq!(parsed.tier, metadata.tier);
        assert!((parsed.priority - 0.456789).abs() < 0.00001);
        assert_eq!(parsed.last_access, metadata.last_access);
    }

    #[test]
    fn kv_metadata_rejects_missing_or_invalid_fields() {
        assert!(matches!(
            deserialize_kv_metadata("id_hex=61 tier=cold checksum=1 priority=0.1 last_access=1"),
            Err(MemoryError::InvalidInput(_))
        ));
        assert!(matches!(
            deserialize_kv_metadata(
                "id_hex=zz byte_len=1 checksum=1 tier=cold priority=0.1 last_access=1"
            ),
            Err(MemoryError::InvalidInput(_))
        ));
        assert!(matches!(
            deserialize_kv_metadata(
                "id_hex=61 byte_len=1 checksum=1 tier=archive priority=0.1 last_access=1"
            ),
            Err(MemoryError::InvalidInput(_))
        ));
    }

    #[test]
    fn hot_metadata_clamps_priority_and_tracks_access_order() {
        let backend = InMemoryDiskKvOffload::new();
        let mut swap = KvSwapManager::new(backend);

        swap.stage_hot("first".to_owned(), vec![1, 2], 9.0).unwrap();
        swap.stage_hot("second".to_owned(), vec![3, 4, 5], -1.0)
            .unwrap();

        let first = swap.metadata("first").unwrap();
        let second = swap.metadata("second").unwrap();
        assert_eq!(first.tier, KvTier::Hot);
        assert_eq!(first.priority, 1.0);
        assert_eq!(first.byte_len, 2);
        assert!(first.checksum != 0);
        assert_eq!(second.priority, 0.0);
        assert!(second.last_access > first.last_access);
    }

    #[test]
    fn stage_hot_replaces_matching_cold_shard_without_stale_catalog() {
        let backend = InMemoryDiskKvOffload::new();
        let mut swap = KvSwapManager::new(backend);

        swap.stage_hot("same".to_owned(), vec![1, 2], 0.1).unwrap();
        let plan = swap.plan_eviction(0);
        assert_eq!(swap.evict(&plan).unwrap()[0].tier, KvTier::Cold);
        assert!(swap.backend().cold_metadata("same").is_some());

        swap.stage_hot("same".to_owned(), vec![7, 8, 9], 0.8)
            .unwrap();

        assert!(swap.backend().cold_metadata("same").is_none());
        assert_eq!(swap.hot_bytes("same"), Some([7, 8, 9].as_slice()));
        let metadata = swap.metadata("same").unwrap();
        assert_eq!(metadata.tier, KvTier::Hot);
        assert_eq!(metadata.byte_len, 3);
        assert_eq!(metadata.priority, 0.8);

        let snapshot = swap.state_snapshot();
        assert_eq!(snapshot.hot_shard_count, 1);
        assert_eq!(snapshot.cold_shard_count, 0);
        assert_eq!(snapshot.metadata_count, 1);
        assert_eq!(snapshot.hot_byte_len, 3);
        assert_eq!(snapshot.cold_byte_len, 0);
        assert_eq!(
            snapshot.shape_codes(),
            vec![
                "hot_metadata".to_owned(),
                "hot_only".to_owned(),
                "metadata_index".to_owned()
            ]
        );

        let prefetch = swap.plan_prefetch(&["same".to_owned()]);
        assert!(prefetch.promote_ids.is_empty());
        assert_eq!(prefetch.already_hot_ids, vec!["same".to_owned()]);
    }

    #[test]
    fn eviction_boundary_keeps_hot_when_target_is_already_met() {
        let backend = InMemoryDiskKvOffload::new();
        let mut swap = KvSwapManager::new(backend);
        swap.stage_hot("a".to_owned(), vec![1, 2, 3], 0.2).unwrap();

        let plan = swap.plan_eviction(3);
        assert!(plan.demote_ids.is_empty());
        assert_eq!(plan.keep_hot_ids, vec!["a".to_owned()]);
        assert_eq!(plan.demote_count(), 0);
        assert_eq!(plan.keep_hot_count(), 1);
        assert_eq!(
            plan.reason_codes(),
            vec!["evict_keep_hot".to_owned(), "target_hot_bytes".to_owned()]
        );
        assert_eq!(
            plan.summary_line(),
            "kvswap_eviction target_hot_bytes=3 demote=0 keep_hot=1 reason=target_hot_bytes demote_id_hex=none keep_hot_id_hex=61 reason_codes=evict_keep_hot|target_hot_bytes detail_codes=keep_hot:61"
        );
        assert!(swap.evict(&plan).unwrap().is_empty());
        assert_eq!(swap.hot_byte_len(), 3);
    }

    #[test]
    fn eviction_prefers_low_priority_then_older_hot_shards() {
        let backend = InMemoryDiskKvOffload::new();
        let mut swap = KvSwapManager::new(backend);
        swap.stage_hot("old".to_owned(), vec![1, 1, 1], 0.4)
            .unwrap();
        swap.stage_hot("new".to_owned(), vec![2, 2, 2], 0.4)
            .unwrap();
        swap.stage_hot("important".to_owned(), vec![3, 3, 3], 0.9)
            .unwrap();

        let plan = swap.plan_eviction(6);
        assert_eq!(plan.demote_ids, vec!["old".to_owned()]);
        assert_eq!(
            plan.keep_hot_ids,
            vec!["important".to_owned(), "new".to_owned()]
        );
        assert_eq!(
            plan.summary_line(),
            "kvswap_eviction target_hot_bytes=6 demote=1 keep_hot=2 reason=target_hot_bytes demote_id_hex=6f6c64 keep_hot_id_hex=696d706f7274616e74|6e6577 reason_codes=evict_demote|evict_keep_hot|target_hot_bytes detail_codes=demote:target_hot_bytes:6f6c64|keep_hot:696d706f7274616e74|keep_hot:6e6577"
        );
    }

    #[test]
    fn kvswap_execution_dedupes_manual_eviction_and_prefetch_boundaries() {
        let backend = InMemoryDiskKvOffload::new();
        let mut swap = KvSwapManager::new(backend);
        swap.stage_hot("a".to_owned(), vec![1, 2], 0.2).unwrap();
        swap.stage_hot("b".to_owned(), vec![3], 0.9).unwrap();
        let eviction = KvEvictionPlan {
            demote_ids: vec!["a".to_owned(), "a".to_owned(), "missing".to_owned()],
            keep_hot_ids: vec!["b".to_owned()],
            target_hot_bytes: 1,
            reason: "manual_review".to_owned(),
        };

        let demoted = swap.evict(&eviction).unwrap();

        assert_eq!(demoted.len(), 1);
        assert_eq!(demoted[0].id, "a");
        assert_eq!(demoted[0].tier, KvTier::Cold);
        assert!(swap.hot_bytes("a").is_none());
        assert_eq!(swap.hot_bytes("b"), Some([3].as_slice()));
        assert!(swap.backend().cold_metadata("a").is_some());
        assert!(swap.metadata("missing").is_none());

        let prefetch = KvPrefetchPlan {
            promote_ids: vec!["a".to_owned(), "a".to_owned(), "missing".to_owned()],
            missing_ids: Vec::new(),
            already_hot_ids: Vec::new(),
            duplicate_ids: vec!["a".to_owned()],
            reason: "manual_review".to_owned(),
        };

        assert_eq!(swap.prefetch(&prefetch).unwrap(), vec!["a".to_owned()]);
        assert_eq!(swap.hot_bytes("a"), Some([1, 2].as_slice()));
        assert!(swap.backend().cold_metadata("a").is_none());
        assert!(swap.metadata("missing").is_none());
        assert_eq!(swap.state_snapshot().hot_shard_count, 2);
        assert_eq!(swap.state_snapshot().cold_shard_count, 0);
        assert_eq!(swap.state_snapshot().metadata_count, 2);
    }

    #[test]
    fn cold_shards_are_listed_and_removed_after_prefetch() {
        let backend = InMemoryDiskKvOffload::new();
        let mut swap = KvSwapManager::new(backend);
        swap.stage_hot("cold".to_owned(), b"cold bytes".to_vec(), 0.1)
            .unwrap();
        swap.stage_hot("hot".to_owned(), b"hot bytes".to_vec(), 0.9)
            .unwrap();

        let plan = swap.plan_eviction(9);
        assert_eq!(plan.demote_ids, vec!["cold".to_owned()]);
        swap.evict(&plan).unwrap();
        assert_eq!(swap.backend().list_cold_metadata().len(), 1);

        let prefetch =
            swap.plan_prefetch(&["hot".to_owned(), "cold".to_owned(), "missing".to_owned()]);
        assert_eq!(prefetch.promote_ids, vec!["cold".to_owned()]);
        assert_eq!(prefetch.missing_ids, vec!["missing".to_owned()]);
        assert_eq!(prefetch.already_hot_ids, vec!["hot".to_owned()]);
        assert!(prefetch.duplicate_ids.is_empty());
        assert_eq!(prefetch.promote_count(), 1);
        assert_eq!(prefetch.missing_count(), 1);
        assert_eq!(prefetch.already_hot_count(), 1);
        assert_eq!(prefetch.duplicate_count(), 0);
        assert_eq!(
            prefetch.reason_codes(),
            vec![
                "prefetch_already_hot".to_owned(),
                "prefetch_missing".to_owned(),
                "prefetch_promote".to_owned(),
                "requested_ids".to_owned(),
            ]
        );
        assert_eq!(
            prefetch.summary_line(),
            "kvswap_prefetch promote=1 missing=1 hot=1 duplicate=0 reason=requested_ids promote_id_hex=636f6c64 missing_id_hex=6d697373696e67 hot_id_hex=686f74 duplicate_id_hex=none reason_codes=prefetch_already_hot|prefetch_missing|prefetch_promote|requested_ids detail_codes=already_hot:686f74|missing:requested_ids:6d697373696e67|promote:requested_ids:636f6c64"
        );
        assert_eq!(swap.prefetch(&prefetch).unwrap(), vec!["cold".to_owned()]);
        assert!(swap.backend().cold_metadata("cold").is_none());
        assert_eq!(swap.hot_bytes("cold"), Some(b"cold bytes".as_slice()));
    }

    #[test]
    fn kvswap_state_snapshot_reports_hot_and_cold_metadata_without_bytes() {
        let backend = InMemoryDiskKvOffload::new();
        let mut swap = KvSwapManager::new(backend);
        assert!(swap.state_snapshot().is_empty());
        assert_eq!(
            swap.state_snapshot().shape_codes(),
            vec!["empty".to_owned()]
        );
        assert_eq!(
            swap.state_snapshot().summary_line(),
            "kvswap_state empty=true hot=0 cold=0 metadata=0 hot_bytes=0 cold_bytes=0 total_bytes=0 shape_codes=empty"
        );

        swap.stage_hot("cold".to_owned(), b"cold".to_vec(), 0.1)
            .unwrap();
        swap.stage_hot("hot".to_owned(), b"hot".to_vec(), 0.9)
            .unwrap();
        let eviction = swap.plan_eviction(3);
        assert_eq!(eviction.demote_ids, vec!["cold".to_owned()]);
        swap.evict(&eviction).unwrap();

        let snapshot = swap.state_snapshot();
        assert_eq!(snapshot.hot_shard_count, 1);
        assert_eq!(snapshot.cold_shard_count, 1);
        assert_eq!(snapshot.metadata_count, 2);
        assert_eq!(snapshot.hot_byte_len, 3);
        assert_eq!(snapshot.cold_byte_len, 4);
        assert_eq!(snapshot.total_byte_len(), 7);
        assert_eq!(
            snapshot.shape_codes(),
            vec![
                "cold_catalog".to_owned(),
                "hot_metadata".to_owned(),
                "metadata_index".to_owned(),
                "mixed_tiers".to_owned(),
            ]
        );
        assert_eq!(
            snapshot.summary_line(),
            "kvswap_state empty=false hot=1 cold=1 metadata=2 hot_bytes=3 cold_bytes=4 total_bytes=7 shape_codes=cold_catalog|hot_metadata|metadata_index|mixed_tiers"
        );

        let prefetch = swap.plan_prefetch(&["cold".to_owned()]);
        assert_eq!(swap.prefetch(&prefetch).unwrap(), vec!["cold".to_owned()]);
        assert_eq!(
            swap.state_snapshot().shape_codes(),
            vec![
                "hot_metadata".to_owned(),
                "hot_only".to_owned(),
                "metadata_index".to_owned(),
            ]
        );
        assert_eq!(
            swap.state_snapshot().summary_line(),
            "kvswap_state empty=false hot=2 cold=0 metadata=2 hot_bytes=7 cold_bytes=0 total_bytes=7 shape_codes=hot_metadata|hot_only|metadata_index"
        );
    }

    #[test]
    fn kvswap_state_snapshot_reports_cold_only_after_full_eviction() {
        let backend = InMemoryDiskKvOffload::new();
        let mut swap = KvSwapManager::new(backend);
        swap.stage_hot("older".to_owned(), b"older bytes".to_vec(), 0.1)
            .unwrap();
        swap.stage_hot("newer".to_owned(), b"newer".to_vec(), 0.2)
            .unwrap();

        let eviction = swap.plan_eviction(0);
        assert_eq!(
            eviction.demote_ids,
            vec!["older".to_owned(), "newer".to_owned()]
        );
        assert_eq!(swap.evict(&eviction).unwrap().len(), 2);

        let snapshot = swap.state_snapshot();
        assert_eq!(snapshot.hot_shard_count, 0);
        assert_eq!(snapshot.cold_shard_count, 2);
        assert_eq!(snapshot.metadata_count, 2);
        assert_eq!(snapshot.hot_byte_len, 0);
        assert_eq!(snapshot.cold_byte_len, 16);
        assert_eq!(
            snapshot.shape_codes(),
            vec![
                "cold_catalog".to_owned(),
                "cold_only".to_owned(),
                "metadata_index".to_owned(),
            ]
        );
        assert_eq!(
            snapshot.summary_line(),
            "kvswap_state empty=false hot=0 cold=2 metadata=2 hot_bytes=0 cold_bytes=16 total_bytes=16 shape_codes=cold_catalog|cold_only|metadata_index"
        );
    }

    #[test]
    fn kvswap_plan_summary_hex_encodes_task_scoped_shard_ids() {
        let backend = InMemoryDiskKvOffload::new();
        let mut swap = KvSwapManager::new(backend);
        swap.stage_hot("task/a shard".to_owned(), b"cold bytes".to_vec(), 0.1)
            .unwrap();
        swap.stage_hot("task/hot shard".to_owned(), b"hot".to_vec(), 0.9)
            .unwrap();

        let eviction = swap.plan_eviction(3);
        assert_eq!(eviction.demote_ids, vec!["task/a shard".to_owned()]);
        assert_eq!(
            eviction.summary_line(),
            "kvswap_eviction target_hot_bytes=3 demote=1 keep_hot=1 reason=target_hot_bytes demote_id_hex=7461736b2f61207368617264 keep_hot_id_hex=7461736b2f686f74207368617264 reason_codes=evict_demote|evict_keep_hot|target_hot_bytes detail_codes=demote:target_hot_bytes:7461736b2f61207368617264|keep_hot:7461736b2f686f74207368617264"
        );
        assert_eq!(
            eviction.detail_codes(),
            vec![
                "demote:target_hot_bytes:7461736b2f61207368617264".to_owned(),
                "keep_hot:7461736b2f686f74207368617264".to_owned()
            ]
        );
        assert_eq!(
            eviction.detail_codes_for_action("demote"),
            vec!["demote:target_hot_bytes:7461736b2f61207368617264".to_owned()]
        );
        assert_eq!(
            eviction.detail_codes_for_action("keep_hot"),
            vec!["keep_hot:7461736b2f686f74207368617264".to_owned()]
        );
        assert_eq!(
            eviction.detail_codes_for_reason("target_hot_bytes"),
            vec!["demote:target_hot_bytes:7461736b2f61207368617264".to_owned()]
        );
        swap.evict(&eviction).unwrap();

        let prefetch =
            swap.plan_prefetch(&["task/a shard".to_owned(), "task/missing shard".to_owned()]);
        assert_eq!(
            prefetch.summary_line(),
            "kvswap_prefetch promote=1 missing=1 hot=0 duplicate=0 reason=requested_ids promote_id_hex=7461736b2f61207368617264 missing_id_hex=7461736b2f6d697373696e67207368617264 hot_id_hex=none duplicate_id_hex=none reason_codes=prefetch_missing|prefetch_promote|requested_ids detail_codes=missing:requested_ids:7461736b2f6d697373696e67207368617264|promote:requested_ids:7461736b2f61207368617264"
        );
        assert_eq!(
            prefetch.detail_codes(),
            vec![
                "missing:requested_ids:7461736b2f6d697373696e67207368617264".to_owned(),
                "promote:requested_ids:7461736b2f61207368617264".to_owned()
            ]
        );
        assert_eq!(
            prefetch.detail_codes_for_action("promote"),
            vec!["promote:requested_ids:7461736b2f61207368617264".to_owned()]
        );
        assert_eq!(
            prefetch.detail_codes_for_action("missing"),
            vec!["missing:requested_ids:7461736b2f6d697373696e67207368617264".to_owned()]
        );
        assert_eq!(
            prefetch.detail_codes_for_reason("requested_ids"),
            vec![
                "missing:requested_ids:7461736b2f6d697373696e67207368617264".to_owned(),
                "promote:requested_ids:7461736b2f61207368617264".to_owned()
            ]
        );
    }

    #[test]
    fn prefetch_missing_cold_bytes_does_not_create_hot_state() {
        let backend = InMemoryDiskKvOffload::new();
        let mut swap = KvSwapManager::new(backend);
        swap.stage_hot("cold".to_owned(), b"cold bytes".to_vec(), 0.7)
            .unwrap();
        let eviction = swap.plan_eviction(0);
        swap.evict(&eviction).unwrap();
        let plan = swap.plan_prefetch(&["cold".to_owned()]);
        swap.backend_mut().delete_cold_shard("cold").unwrap();

        assert_eq!(plan.promote_ids, vec!["cold".to_owned()]);
        assert!(swap.prefetch(&plan).unwrap().is_empty());
        assert!(swap.hot_bytes("cold").is_none());
        assert_eq!(swap.metadata("cold").unwrap().tier, KvTier::Cold);
    }

    #[test]
    fn prefetch_uses_backend_cold_catalog_when_manager_metadata_is_empty() {
        let mut backend = InMemoryDiskKvOffload::new();
        backend
            .write_cold_shard(ColdKvShard {
                metadata: KvShardMetadata {
                    id: "catalog-cold".to_owned(),
                    byte_len: 0,
                    checksum: 0,
                    tier: KvTier::Hot,
                    priority: 0.6,
                    last_access: 17,
                },
                bytes: b"catalog bytes".to_vec(),
            })
            .unwrap();
        let mut swap = KvSwapManager::new(backend);

        assert_eq!(
            swap.state_snapshot().shape_codes(),
            vec![
                "cold_catalog".to_owned(),
                "cold_only".to_owned(),
                "metadata_index".to_owned(),
            ]
        );
        assert_eq!(swap.metadata("catalog-cold").unwrap().tier, KvTier::Cold);
        let plan = swap.plan_prefetch(&["catalog-cold".to_owned()]);

        assert_eq!(plan.promote_ids, vec!["catalog-cold".to_owned()]);
        assert!(plan.missing_ids.is_empty());
        assert_eq!(
            plan.summary_line(),
            "kvswap_prefetch promote=1 missing=0 hot=0 duplicate=0 reason=requested_ids promote_id_hex=636174616c6f672d636f6c64 missing_id_hex=none hot_id_hex=none duplicate_id_hex=none reason_codes=prefetch_promote|requested_ids detail_codes=promote:requested_ids:636174616c6f672d636f6c64"
        );
        assert_eq!(
            swap.prefetch(&plan).unwrap(),
            vec!["catalog-cold".to_owned()]
        );
        assert_eq!(
            swap.hot_bytes("catalog-cold"),
            Some(b"catalog bytes".as_slice())
        );
        assert!(swap.backend().cold_metadata("catalog-cold").is_none());
        assert_eq!(swap.metadata("catalog-cold").unwrap().tier, KvTier::Hot);
    }

    #[test]
    fn prefetch_recomputes_hot_metadata_from_cold_bytes() {
        let mut backend = InMemoryDiskKvOffload::new();
        backend.shards.insert(
            "stale".to_owned(),
            ColdKvShard {
                metadata: KvShardMetadata {
                    id: "stale".to_owned(),
                    byte_len: 999,
                    checksum: 7,
                    tier: KvTier::Cold,
                    priority: 0.6,
                    last_access: 17,
                },
                bytes: b"actual bytes".to_vec(),
            },
        );
        let mut swap = KvSwapManager::new(backend);
        let plan = swap.plan_prefetch(&["stale".to_owned()]);

        assert_eq!(plan.promote_ids, vec!["stale".to_owned()]);
        assert_eq!(swap.prefetch(&plan).unwrap(), vec!["stale".to_owned()]);

        let hot = swap.metadata("stale").unwrap();
        assert_eq!(hot.tier, KvTier::Hot);
        assert_eq!(hot.byte_len, b"actual bytes".len());
        assert_eq!(hot.checksum, checksum(b"actual bytes"));
        assert_eq!(hot.priority, 0.6);
        assert!(hot.last_access > 17);
        assert_eq!(swap.hot_bytes("stale"), Some(b"actual bytes".as_slice()));
        assert!(swap.backend().cold_metadata("stale").is_none());
    }

    #[test]
    fn prefetch_refreshes_hot_metadata_and_removes_cold_catalog_entry() {
        let backend = InMemoryDiskKvOffload::new();
        let mut swap = KvSwapManager::new(backend);
        swap.stage_hot("warm".to_owned(), b"warm bytes".to_vec(), 0.42)
            .unwrap();
        let original = swap.metadata("warm").unwrap();
        let eviction = swap.plan_eviction(0);
        swap.evict(&eviction).unwrap();
        let cold = swap.metadata("warm").unwrap();
        swap.stage_hot("other".to_owned(), b"other".to_vec(), 0.9)
            .unwrap();

        let plan = swap.plan_prefetch(&["warm".to_owned()]);
        assert_eq!(plan.promote_ids, vec!["warm".to_owned()]);
        assert_eq!(swap.prefetch(&plan).unwrap(), vec!["warm".to_owned()]);

        let hot = swap.metadata("warm").unwrap();
        assert_eq!(cold.tier, KvTier::Cold);
        assert_eq!(hot.tier, KvTier::Hot);
        assert_eq!(hot.byte_len, original.byte_len);
        assert_eq!(hot.checksum, original.checksum);
        assert_eq!(hot.priority, original.priority);
        assert!(hot.last_access > cold.last_access);
        assert_eq!(swap.hot_bytes("warm"), Some(b"warm bytes".as_slice()));
        assert!(swap.backend().cold_metadata("warm").is_none());
        assert_eq!(
            swap.state_snapshot().shape_codes(),
            vec![
                "hot_metadata".to_owned(),
                "hot_only".to_owned(),
                "metadata_index".to_owned()
            ]
        );
    }

    #[test]
    fn prefetch_plan_dedupes_repeated_ids_and_reports_hot_boundary() {
        let backend = InMemoryDiskKvOffload::new();
        let mut swap = KvSwapManager::new(backend);
        swap.stage_hot("cold".to_owned(), b"cold bytes".to_vec(), 0.7)
            .unwrap();
        swap.stage_hot("hot".to_owned(), b"hot bytes".to_vec(), 0.9)
            .unwrap();
        let eviction = swap.plan_eviction(9);
        swap.evict(&eviction).unwrap();

        let plan = swap.plan_prefetch(&[
            "cold".to_owned(),
            "cold".to_owned(),
            "hot".to_owned(),
            "hot".to_owned(),
            "missing".to_owned(),
            "missing".to_owned(),
        ]);

        assert_eq!(plan.promote_ids, vec!["cold".to_owned()]);
        assert_eq!(plan.already_hot_ids, vec!["hot".to_owned()]);
        assert_eq!(plan.missing_ids, vec!["missing".to_owned()]);
        assert_eq!(
            plan.duplicate_ids,
            vec!["cold".to_owned(), "hot".to_owned(), "missing".to_owned()]
        );
        assert_eq!(
            plan.reason_codes(),
            vec![
                "prefetch_already_hot".to_owned(),
                "prefetch_duplicate".to_owned(),
                "prefetch_missing".to_owned(),
                "prefetch_promote".to_owned(),
                "requested_ids".to_owned(),
            ]
        );
        assert_eq!(
            plan.detail_codes(),
            vec![
                "already_hot:686f74".to_owned(),
                "duplicate:636f6c64".to_owned(),
                "duplicate:686f74".to_owned(),
                "duplicate:6d697373696e67".to_owned(),
                "missing:requested_ids:6d697373696e67".to_owned(),
                "promote:requested_ids:636f6c64".to_owned(),
            ]
        );
        assert_eq!(swap.prefetch(&plan).unwrap(), vec!["cold".to_owned()]);
        assert_eq!(swap.hot_bytes("cold"), Some(b"cold bytes".as_slice()));
    }

    #[test]
    fn kvswap_manager_reports_adapter_health_with_unique_shards() {
        let backend = InMemoryDiskKvOffload::new();
        let mut swap = KvSwapManager::new(backend);
        swap.stage_hot("a".to_owned(), vec![1], 0.1).unwrap();
        swap.stage_hot("b".to_owned(), vec![2], 0.1).unwrap();
        let plan = swap.plan_eviction(1);
        swap.evict(&plan).unwrap();

        let descriptor = swap.descriptor();
        assert_eq!(descriptor.name, "kvswap_manager");
        assert!(
            descriptor
                .capabilities
                .contains(&MemoryAdapterCapability::KvSwap)
        );
        assert!(
            descriptor
                .capabilities
                .contains(&MemoryAdapterCapability::DiskKvOffload)
        );
        assert_eq!(swap.health().unwrap().record_count, Some(2));
    }

    #[test]
    fn empty_shard_ids_are_rejected_before_offload() {
        let backend = InMemoryDiskKvOffload::new();
        let mut swap = KvSwapManager::new(backend);

        let error = swap.stage_hot(" ".to_owned(), vec![1], 0.5).unwrap_err();
        assert!(matches!(error, MemoryError::InvalidInput(_)));
    }

    #[test]
    fn keyspace_hex_encodes_shard_ids_for_storage_keys() {
        let keyspace = DiskKvShardKeyspace::new("kv/");
        let keys = keyspace.keys_for("agent:session 1/kv").unwrap();

        assert_eq!(
            keys.bytes_key,
            "kv/6167656e743a73657373696f6e20312f6b76/bytes"
        );
        assert_eq!(
            keyspace.shard_id_from_key(&keys.metadata_key).as_deref(),
            Some("agent:session 1/kv")
        );
        assert_eq!(keyspace.shard_id_from_key("kv/not-hex/bytes"), None);
    }

    #[test]
    fn kv_metadata_serializes_and_deserializes_stable_manifest_line() {
        let metadata = KvShardMetadata {
            id: "cold:agent/1".to_owned(),
            byte_len: 42,
            checksum: 99,
            tier: KvTier::Cold,
            priority: 1.5,
            last_access: 7,
        };

        let serialized = serialize_kv_metadata(&metadata);
        let decoded = deserialize_kv_metadata(&serialized).unwrap();

        assert_eq!(decoded.id, metadata.id);
        assert_eq!(decoded.byte_len, 42);
        assert_eq!(decoded.checksum, 99);
        assert_eq!(decoded.tier, KvTier::Cold);
        assert_eq!(decoded.priority, 1.0);
        assert_eq!(decoded.last_access, 7);
    }

    #[test]
    fn shard_manifest_pairs_keyspace_keys_with_metadata_line() {
        let keyspace = DiskKvShardKeyspace::default();
        let metadata = KvShardMetadata {
            id: "task/shard 1".to_owned(),
            byte_len: 7,
            checksum: 42,
            tier: KvTier::Cold,
            priority: 0.8,
            last_access: 9,
        };

        let manifest = keyspace.manifest_for(metadata.clone()).unwrap();

        assert_eq!(manifest.metadata, metadata);
        assert_eq!(
            manifest.keys.bytes_key,
            "kvswap/shard/7461736b2f73686172642031/bytes"
        );
        assert_eq!(
            manifest.keys.metadata_key,
            "kvswap/shard/7461736b2f73686172642031/metadata"
        );
        assert_eq!(
            deserialize_kv_metadata(&manifest.metadata_line()).unwrap(),
            manifest.metadata
        );
    }

    #[test]
    fn catalog_manifests_extract_metadata_from_mixed_entries() {
        let keyspace = DiskKvShardKeyspace::default();
        let cold = KvShardMetadata {
            id: "cold".to_owned(),
            byte_len: 4,
            checksum: 1,
            tier: KvTier::Cold,
            priority: 0.4,
            last_access: 2,
        };
        let hot = KvShardMetadata {
            id: "hot".to_owned(),
            byte_len: 8,
            checksum: 2,
            tier: KvTier::Hot,
            priority: 0.9,
            last_access: 3,
        };
        let cold_keys = keyspace.keys_for("cold").unwrap();
        let hot_keys = keyspace.keys_for("hot").unwrap();
        let cold_line = serialize_kv_metadata(&cold);
        let hot_line = serialize_kv_metadata(&hot);
        let entries = vec![
            (cold_keys.bytes_key.as_str(), "raw bytes are ignored"),
            (hot_keys.metadata_key.as_str(), hot_line.as_str()),
            ("other/prefix", "ignored"),
            (cold_keys.metadata_key.as_str(), cold_line.as_str()),
        ];

        let manifests = keyspace.catalog_manifests(entries).unwrap();

        assert_eq!(manifests.len(), 2);
        assert_eq!(manifests[0].metadata.id, "cold");
        assert_eq!(manifests[1].metadata.id, "hot");
    }

    #[test]
    fn catalog_verification_confirms_copied_fixture_bytes() {
        let keyspace = DiskKvShardKeyspace::default();
        let bytes = b"cold bytes".to_vec();
        let metadata = KvShardMetadata {
            id: "cold".to_owned(),
            byte_len: bytes.len(),
            checksum: checksum(&bytes),
            tier: KvTier::Cold,
            priority: 0.7,
            last_access: 4,
        };
        let keys = keyspace.keys_for("cold").unwrap();
        let line = serialize_kv_metadata(&metadata);
        let entries = vec![
            (keys.metadata_key.as_str(), line.as_bytes()),
            (keys.bytes_key.as_str(), bytes.as_slice()),
            ("unrelated/key", b"ignored".as_slice()),
        ];

        let verification = keyspace.verify_catalog_entries(entries).unwrap();

        assert!(verification.is_verified());
        assert!(verification.catalog_verified());
        assert!(verification.checksum_verified());
        assert_eq!(verification.record_count(), 1);
        assert_eq!(verification.manifests[0].metadata.id, "cold");
        assert_eq!(verification.reason_codes(), Vec::<String>::new());
        assert_eq!(verification.detail_codes(), Vec::<String>::new());
        assert_eq!(
            verification.summary_line(),
            "disk_kv_catalog verified=true catalog_verified=true checksum_verified=true records=1 missing_bytes=0 byte_len_mismatches=0 checksum_mismatches=0 missing_id_hex=none byte_len_mismatch_id_hex=none checksum_mismatch_id_hex=none reason_codes=none detail_codes=none"
        );
    }

    #[test]
    fn catalog_verification_reports_missing_and_mismatched_bytes() {
        let keyspace = DiskKvShardKeyspace::default();
        let missing = KvShardMetadata {
            id: "missing".to_owned(),
            byte_len: 4,
            checksum: 42,
            tier: KvTier::Cold,
            priority: 0.2,
            last_access: 1,
        };
        let corrupt_bytes = b"corrupt bytes".to_vec();
        let corrupt = KvShardMetadata {
            id: "corrupt".to_owned(),
            byte_len: 4,
            checksum: checksum(b"expected bytes"),
            tier: KvTier::Cold,
            priority: 0.8,
            last_access: 2,
        };
        let missing_keys = keyspace.keys_for("missing").unwrap();
        let corrupt_keys = keyspace.keys_for("corrupt").unwrap();
        let missing_line = serialize_kv_metadata(&missing);
        let corrupt_line = serialize_kv_metadata(&corrupt);
        let entries = vec![
            (missing_keys.metadata_key.as_str(), missing_line.as_bytes()),
            (corrupt_keys.metadata_key.as_str(), corrupt_line.as_bytes()),
            (corrupt_keys.bytes_key.as_str(), corrupt_bytes.as_slice()),
        ];

        let verification = keyspace.verify_catalog_entries(entries).unwrap();

        assert!(!verification.is_verified());
        assert!(!verification.catalog_verified());
        assert!(!verification.checksum_verified());
        assert_eq!(verification.missing_byte_ids, vec!["missing".to_owned()]);
        assert_eq!(
            verification.byte_len_mismatch_ids,
            vec!["corrupt".to_owned()]
        );
        assert_eq!(
            verification.checksum_mismatch_ids,
            vec!["corrupt".to_owned()]
        );
        assert_eq!(
            verification.reason_codes(),
            vec![
                "byte_len_mismatch".to_owned(),
                "checksum_mismatch".to_owned(),
                "missing_bytes".to_owned(),
            ]
        );
        assert_eq!(
            verification.detail_codes(),
            vec![
                "byte_len_mismatch:636f7272757074".to_owned(),
                "checksum_mismatch:636f7272757074".to_owned(),
                "missing_bytes:6d697373696e67".to_owned(),
            ]
        );
        assert_eq!(
            verification.summary_line(),
            "disk_kv_catalog verified=false catalog_verified=false checksum_verified=false records=2 missing_bytes=1 byte_len_mismatches=1 checksum_mismatches=1 missing_id_hex=6d697373696e67 byte_len_mismatch_id_hex=636f7272757074 checksum_mismatch_id_hex=636f7272757074 reason_codes=byte_len_mismatch|checksum_mismatch|missing_bytes detail_codes=byte_len_mismatch:636f7272757074|checksum_mismatch:636f7272757074|missing_bytes:6d697373696e67"
        );
    }

    #[test]
    fn catalog_manifest_rejects_key_metadata_id_mismatch() {
        let keyspace = DiskKvShardKeyspace::default();
        let keys = keyspace.keys_for("key-id").unwrap();
        let metadata = KvShardMetadata {
            id: "metadata-id".to_owned(),
            byte_len: 1,
            checksum: 1,
            tier: KvTier::Cold,
            priority: 0.5,
            last_access: 1,
        };

        let error = keyspace
            .metadata_manifest_from_entry(&keys.metadata_key, &serialize_kv_metadata(&metadata))
            .unwrap_err();

        assert!(matches!(error, MemoryError::InvalidInput(_)));
    }

    #[test]
    fn catalog_manifest_rejects_duplicate_metadata_entries() {
        let keyspace = DiskKvShardKeyspace::default();
        let keys = keyspace.keys_for("dup").unwrap();
        let metadata = KvShardMetadata {
            id: "dup".to_owned(),
            byte_len: 1,
            checksum: 1,
            tier: KvTier::Cold,
            priority: 0.5,
            last_access: 1,
        };
        let line = serialize_kv_metadata(&metadata);
        let entries = vec![
            (keys.metadata_key.as_str(), line.as_str()),
            (keys.metadata_key.as_str(), line.as_str()),
        ];

        let error = keyspace.catalog_manifests(entries).unwrap_err();

        assert!(matches!(error, MemoryError::InvalidInput(_)));
    }

    #[test]
    fn kv_metadata_rejects_missing_or_invalid_manifest_fields() {
        let missing = deserialize_kv_metadata("id_hex=6162 tier=cold").unwrap_err();
        assert!(matches!(missing, MemoryError::InvalidInput(_)));

        let invalid_hex = deserialize_kv_metadata(
            "id_hex=not-hex byte_len=1 checksum=2 tier=cold priority=0.5 last_access=3",
        )
        .unwrap_err();
        assert!(matches!(invalid_hex, MemoryError::InvalidInput(_)));
    }

    #[test]
    fn file_offload_round_trips_cold_shards() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("file-offload-test");
        if root.exists() {
            std::fs::remove_dir_all(&root).unwrap();
        }
        let mut offload = FileDiskKvOffload::open(&root).unwrap();
        let metadata = KvShardMetadata {
            id: "cold-a".to_owned(),
            byte_len: 0,
            checksum: 0,
            tier: KvTier::Hot,
            priority: 0.4,
            last_access: 7,
        };
        offload
            .write_cold_shard(ColdKvShard {
                metadata,
                bytes: b"cold bytes".to_vec(),
            })
            .unwrap();
        let shard = offload.read_cold_shard("cold-a").unwrap().unwrap();
        assert_eq!(shard.bytes, b"cold bytes");
        assert_eq!(shard.metadata.tier, KvTier::Cold);
        assert!(offload.delete_cold_shard("cold-a").unwrap());
    }
}
