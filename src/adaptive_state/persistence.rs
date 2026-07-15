mod ledger_codec;
mod policy_codec;
mod state_codec;

use std::io;
use std::path::Path;

use crate::disk_kv::DiskKvStore;
use crate::kv_cache::{MemoryCompactionPolicy, MemoryRetentionPolicy};
use crate::reasoning_genome::{DnaGeneChain, ReasoningGene, ReasoningGenome};
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

use super::model::{all_profiles, profile_slug};
use super::{AdaptiveState, EvolutionLedger, GenomeProfileState, GenomeRuntimeState};

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
        save_genome_runtime(&mut store, &self.genome_runtime)?;
        store.compact()
    }

    pub fn load_from_disk_kv(path: impl AsRef<Path>) -> io::Result<Option<Self>> {
        let path = path.as_ref();
        if !path.exists() && !path.with_extension("compact.bak").exists() {
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
        let genome_runtime = load_genome_runtime(&store)?;

        Ok(Some(Self {
            router,
            hierarchy,
            tier_plan,
            memory_retention_policy,
            memory_compaction_policy,
            evolution_ledger,
            genome_runtime,
        }))
    }
}

fn save_genome_runtime(store: &mut DiskKvStore, runtime: &GenomeRuntimeState) -> io::Result<()> {
    for profile in all_profiles() {
        let state = runtime.profile(profile);
        let prefix = format!("adaptive/genome/{}", profile_slug(profile));
        store.put(
            format!("{prefix}/active"),
            chain_to_lines(&state.active_chain)?.join("\n"),
        )?;
        store.put(format!("{prefix}/generation"), state.generation.to_string())?;
        store.put(format!("{prefix}/journal"), state.journal_lines.join("\n"))?;
        let previous_key = format!("{prefix}/previous");
        if let Some(previous_chain) = &state.previous_chain {
            store.put(previous_key, chain_to_lines(previous_chain)?.join("\n"))?;
        } else {
            store.delete(&previous_key)?;
        }
    }
    Ok(())
}

fn load_genome_runtime(store: &DiskKvStore) -> io::Result<GenomeRuntimeState> {
    let mut runtime = GenomeRuntimeState::default();
    for profile in all_profiles() {
        let prefix = format!("adaptive/genome/{}", profile_slug(profile));
        let active_key = format!("{prefix}/active");
        let Some(active_bytes) = store.get(&active_key)? else {
            continue;
        };
        let active_chain = chain_from_bytes(&active_bytes)?;
        let active = genome_from_chain(&active_chain)?;
        if active.profile != profile {
            return Err(invalid_genome_state("active genome profile mismatch"));
        }
        let generation = store
            .get(&format!("{prefix}/generation"))?
            .ok_or_else(|| invalid_genome_state("genome generation missing"))
            .and_then(|bytes| {
                String::from_utf8(bytes)
                    .map_err(|_| invalid_genome_state("genome generation is not UTF-8"))?
                    .parse::<u64>()
                    .map_err(|_| invalid_genome_state("genome generation is invalid"))
            })?;
        let previous_chain = store
            .get(&format!("{prefix}/previous"))?
            .map(|bytes| chain_from_bytes(&bytes))
            .transpose()?;
        let previous = previous_chain.as_ref().map(genome_from_chain).transpose()?;
        if previous
            .as_ref()
            .is_some_and(|genome| genome.profile != profile)
        {
            return Err(invalid_genome_state("previous genome profile mismatch"));
        }
        let journal_lines = store
            .get(&format!("{prefix}/journal"))?
            .map(|bytes| lines_from_bytes(&bytes))
            .transpose()?
            .unwrap_or_default();
        *runtime.profile_mut(profile) = GenomeProfileState {
            profile,
            active,
            previous,
            active_chain,
            previous_chain,
            generation,
            journal_lines,
        };
    }
    Ok(runtime)
}

fn chain_to_lines(chain: &DnaGeneChain) -> io::Result<Vec<String>> {
    chain
        .to_kv_lines()
        .map_err(|_| invalid_genome_state("genome chain serialization failed"))
}

fn chain_from_bytes(bytes: &[u8]) -> io::Result<DnaGeneChain> {
    let lines = lines_from_bytes(bytes)?;
    DnaGeneChain::from_kv_lines(&lines)
        .map_err(|_| invalid_genome_state("genome chain parsing failed"))
}

fn genome_from_chain(chain: &DnaGeneChain) -> io::Result<ReasoningGenome> {
    let genes = chain
        .express_chain
        .iter()
        .map(|record| {
            ReasoningGene::new(
                record.gene_id.clone(),
                record.gene_kind,
                record.label.clone(),
                record.purpose.clone(),
            )
            .with_tags(record.tags.clone())
            .with_health(record.age, record.fitness_score, record.drift_score)
            .with_status(record.status)
        })
        .collect::<Vec<_>>();
    if genes.is_empty() {
        return Err(invalid_genome_state(
            "persisted genome has no expression genes",
        ));
    }
    Ok(ReasoningGenome::new(
        chain.genome_id.clone(),
        chain.profile,
        chain.stable_anchor_id.clone(),
        genes,
    ))
}

fn lines_from_bytes(bytes: &[u8]) -> io::Result<Vec<String>> {
    let value = String::from_utf8(bytes.to_vec())
        .map_err(|_| invalid_genome_state("genome state is not UTF-8"))?;
    Ok(value
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

fn invalid_genome_state(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
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
