mod ledger_codec;
mod policy_codec;
mod state_codec;

use std::io;
use std::path::Path;

use crate::disk_kv::DiskKvStore;
use crate::kv_cache::{MemoryCompactionPolicy, MemoryRetentionPolicy};
use crate::tiered_cache::TieredCachePlan;

pub(super) use ledger_codec::parse_evolution_ledger;
use ledger_codec::serialize_evolution_ledger;
use policy_codec::{
    parse_memory_compaction_policy, parse_memory_retention_policy, parse_tier_plan,
    serialize_memory_compaction_policy, serialize_memory_retention_policy, serialize_tier_plan,
};
use state_codec::{
    parse_hierarchy_state, parse_router_state, serialize_hierarchy_state, serialize_router_state,
};

use super::{AdaptiveState, EvolutionLedger};

impl AdaptiveState {
    pub fn save_to_disk_kv(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let mut store = DiskKvStore::open(path)?;
        store.put(
            "adaptive/router",
            serialize_router_state(self.router).as_bytes(),
        )?;
        store.put(
            "adaptive/hierarchy",
            serialize_hierarchy_state(self.hierarchy).as_bytes(),
        )?;
        store.put(
            "adaptive/tier_plan",
            serialize_tier_plan(&self.tier_plan).as_bytes(),
        )?;
        store.put(
            "adaptive/memory_retention",
            serialize_memory_retention_policy(self.memory_retention_policy).as_bytes(),
        )?;
        store.put(
            "adaptive/memory_compaction",
            serialize_memory_compaction_policy(&self.memory_compaction_policy).as_bytes(),
        )?;
        store.put(
            "adaptive/evolution_ledger",
            serialize_evolution_ledger(self.evolution_ledger).as_bytes(),
        )?;
        store.compact()
    }

    pub fn load_from_disk_kv(path: impl AsRef<Path>) -> io::Result<Option<Self>> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(None);
        }

        let store = DiskKvStore::open(path)?;
        let Some(router_bytes) = store.get("adaptive/router")? else {
            return Ok(None);
        };
        let Some(hierarchy_bytes) = store.get("adaptive/hierarchy")? else {
            return Ok(None);
        };
        let Some(router) = parse_router_state(&String::from_utf8_lossy(&router_bytes)) else {
            return Ok(None);
        };
        let Some(hierarchy) = parse_hierarchy_state(&String::from_utf8_lossy(&hierarchy_bytes))
        else {
            return Ok(None);
        };

        let tier_plan = load_optional_state(
            &store,
            "adaptive/tier_plan",
            parse_tier_plan,
            TieredCachePlan::default,
        )?;
        let memory_retention_policy = load_optional_state(
            &store,
            "adaptive/memory_retention",
            |value| parse_memory_retention_policy(value).unwrap_or_default(),
            MemoryRetentionPolicy::default,
        )?;
        let memory_compaction_policy = load_optional_state(
            &store,
            "adaptive/memory_compaction",
            |value| parse_memory_compaction_policy(value).unwrap_or_default(),
            MemoryCompactionPolicy::default,
        )?;
        let evolution_ledger = load_optional_state(
            &store,
            "adaptive/evolution_ledger",
            |value| parse_evolution_ledger(value).unwrap_or_default(),
            EvolutionLedger::default,
        )?;

        Ok(Some(Self {
            router,
            hierarchy,
            tier_plan,
            memory_retention_policy,
            memory_compaction_policy,
            evolution_ledger,
        }))
    }
}

fn load_optional_state<T>(
    store: &DiskKvStore,
    key: &str,
    parse: impl FnOnce(&str) -> T,
    default: impl FnOnce() -> T,
) -> io::Result<T> {
    store
        .get(key)?
        .map(|bytes| parse(&String::from_utf8_lossy(&bytes)))
        .map_or_else(|| Ok(default()), Ok)
}
