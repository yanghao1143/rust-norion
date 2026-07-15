mod ledger_codec;
mod policy_codec;
mod state_codec;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::disk_kv::DiskKvStore;
use crate::kv_cache::{MemoryCompactionPolicy, MemoryResidencyState, MemoryRetentionPolicy};
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
use super::{
    AdaptiveState, EvolutionLedger, GENOME_PERSISTED_GENE_CAPACITY, GeneUsageRecord,
    GenomeGeneResidency, GenomeProfileState, GenomeRuntimeState,
};

const GENE_RESIDENCY_SCHEMA_VERSION: &str = "gene_residency_v1";
const GENE_RESIDENCY_SCHEMA_KEY: &str = "adaptive/genome/residency_schema";

impl AdaptiveState {
    pub fn save_to_disk_kv(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let path = path.as_ref();
        restore_adaptive_snapshot_backup(path)?;
        let staged_path = adaptive_sidecar_path(path, ".adaptive.next");
        remove_snapshot_artifacts(&staged_path);
        if let Err(error) = write_adaptive_snapshot(self, &staged_path) {
            remove_snapshot_artifacts(&staged_path);
            return Err(error);
        }
        commit_adaptive_snapshot(path, &staged_path)
    }

    pub fn load_from_disk_kv(path: impl AsRef<Path>) -> io::Result<Option<Self>> {
        let path = path.as_ref();
        restore_adaptive_snapshot_backup(path)?;
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

fn write_adaptive_snapshot(state: &AdaptiveState, path: &Path) -> io::Result<()> {
    {
        let mut store = DiskKvStore::open(path)?;
        store.put(
            "adaptive/router",
            serialize_router_state(state.router).as_bytes(),
        )?;
        store.put(
            "adaptive/hierarchy",
            serialize_hierarchy_state(state.hierarchy).as_bytes(),
        )?;
        store.put(
            "adaptive/tier_plan",
            serialize_tier_plan(&state.tier_plan).as_bytes(),
        )?;
        store.put(
            "adaptive/memory_retention",
            serialize_memory_retention_policy(state.memory_retention_policy).as_bytes(),
        )?;
        store.put(
            "adaptive/memory_compaction",
            serialize_memory_compaction_policy(&state.memory_compaction_policy).as_bytes(),
        )?;
        store.put(
            "adaptive/evolution_ledger",
            serialize_evolution_ledger(state.evolution_ledger).as_bytes(),
        )?;
        save_genome_runtime(&mut store, &state.genome_runtime)?;
        store.compact()?;
    }
    Ok(())
}

fn commit_adaptive_snapshot(path: &Path, staged_path: &Path) -> io::Result<()> {
    let backup_path = adaptive_sidecar_path(path, ".adaptive.bak");
    if path.exists() {
        remove_file_if_exists(&backup_path)?;
        fs::rename(path, &backup_path)?;
    }
    if let Err(error) = fs::rename(staged_path, path) {
        if backup_path.exists() {
            let _ = fs::rename(&backup_path, path);
        }
        return Err(error);
    }
    let _ = fs::remove_file(backup_path);
    Ok(())
}

fn restore_adaptive_snapshot_backup(path: &Path) -> io::Result<()> {
    if path.exists() {
        return Ok(());
    }
    let backup_path = adaptive_sidecar_path(path, ".adaptive.bak");
    match fs::rename(backup_path, path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn remove_snapshot_artifacts(path: &Path) {
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(path.with_extension("compact"));
    let _ = fs::remove_file(path.with_extension("compact.bak"));
}

fn remove_file_if_exists(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn adaptive_sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
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
        store.put(
            format!("{prefix}/residency"),
            serialize_gene_residency(&state.gene_residency, &state.active, state.generation)?,
        )?;
        let previous_key = format!("{prefix}/previous");
        let previous_residency_key = format!("{prefix}/previous_residency");
        if let Some(previous_chain) = &state.previous_chain {
            store.put(previous_key, chain_to_lines(previous_chain)?.join("\n"))?;
            let previous = state
                .previous
                .as_ref()
                .ok_or_else(|| invalid_genome_state("previous genome missing"))?;
            let previous_residency = state
                .previous_gene_residency
                .as_ref()
                .ok_or_else(|| invalid_genome_state("previous gene residency missing"))?;
            store.put(
                previous_residency_key,
                serialize_gene_residency(
                    previous_residency,
                    previous,
                    state.generation.saturating_sub(1),
                )?,
            )?;
        } else {
            store.delete(&previous_key)?;
            store.delete(&previous_residency_key)?;
        }
    }
    store.put(GENE_RESIDENCY_SCHEMA_KEY, GENE_RESIDENCY_SCHEMA_VERSION)?;
    Ok(())
}

fn load_genome_runtime(store: &DiskKvStore) -> io::Result<GenomeRuntimeState> {
    let mut runtime = GenomeRuntimeState::default();
    let residency_schema = store.get(GENE_RESIDENCY_SCHEMA_KEY)?;
    let residency_sidecars_present = store
        .keys_with_prefix("adaptive/genome/")
        .iter()
        .any(|key| key.ends_with("/residency") || key.ends_with("/previous_residency"));
    let has_residency_sidecar = match residency_schema {
        None if residency_sidecars_present => {
            return Err(invalid_genome_state(
                "gene residency sidecar exists without schema marker",
            ));
        }
        None => false,
        Some(bytes) if bytes == GENE_RESIDENCY_SCHEMA_VERSION.as_bytes() => true,
        Some(_) => return Err(invalid_genome_state("gene residency schema is invalid")),
    };
    for profile in all_profiles() {
        let prefix = format!("adaptive/genome/{}", profile_slug(profile));
        let active_key = format!("{prefix}/active");
        let Some(active_bytes) = store.get(&active_key)? else {
            if has_residency_sidecar {
                return Err(invalid_genome_state(
                    "active genome missing for residency schema",
                ));
            }
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
        let gene_residency = if has_residency_sidecar {
            let bytes = store
                .get(&format!("{prefix}/residency"))?
                .ok_or_else(|| invalid_genome_state("gene residency sidecar missing"))?;
            parse_gene_residency(&bytes, &active, generation)?
        } else {
            GenomeGeneResidency::for_genome(&active)
        };
        let previous_gene_residency = match &previous {
            Some(previous) if has_residency_sidecar => {
                if generation == 0 {
                    return Err(invalid_genome_state(
                        "previous gene residency requires a positive generation",
                    ));
                }
                let bytes = store
                    .get(&format!("{prefix}/previous_residency"))?
                    .ok_or_else(|| {
                        invalid_genome_state("previous gene residency sidecar missing")
                    })?;
                Some(parse_gene_residency(
                    &bytes,
                    previous,
                    generation.saturating_sub(1),
                )?)
            }
            Some(previous) => Some(GenomeGeneResidency::for_genome(previous)),
            None => None,
        };
        *runtime.profile_mut(profile) = GenomeProfileState {
            profile,
            active,
            previous,
            active_chain,
            previous_chain,
            gene_residency,
            previous_gene_residency,
            generation,
            journal_lines,
        };
    }
    Ok(runtime)
}

fn serialize_gene_residency(
    residency: &GenomeGeneResidency,
    genome: &ReasoningGenome,
    generation: u64,
) -> io::Result<String> {
    validate_gene_residency(residency, genome)?;
    if invalid_sidecar_field(&genome.id) || invalid_sidecar_field(&residency.last_transition_reason)
    {
        return Err(invalid_genome_state("gene residency header is invalid"));
    }
    let mut lines = vec![format!(
        "{}\t{}\t{}\t{}\t{}\t{}",
        GENE_RESIDENCY_SCHEMA_VERSION,
        genome.id,
        generation,
        residency.step,
        residency.last_observation_sequence,
        residency.last_transition_reason
    )];
    lines.extend(residency.records.iter().map(|record| {
        format!(
            "gene\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            record.gene_id,
            record.residency.as_str(),
            record.opportunities,
            record.hits,
            record.failures,
            record.last_used_step,
            record.consumed_evidence_digest
        )
    }));
    Ok(lines.join("\n"))
}

fn parse_gene_residency(
    bytes: &[u8],
    genome: &ReasoningGenome,
    expected_generation: u64,
) -> io::Result<GenomeGeneResidency> {
    let (generation, residency) = parse_gene_residency_header(bytes, genome)?;
    if generation != expected_generation {
        return Err(invalid_genome_state("gene residency generation mismatch"));
    }
    Ok(residency)
}

fn parse_gene_residency_header(
    bytes: &[u8],
    genome: &ReasoningGenome,
) -> io::Result<(u64, GenomeGeneResidency)> {
    let value = String::from_utf8(bytes.to_vec())
        .map_err(|_| invalid_genome_state("gene residency sidecar is not UTF-8"))?;
    let mut lines = value.lines();
    let header = lines
        .next()
        .ok_or_else(|| invalid_genome_state("gene residency header missing"))?
        .split('\t')
        .collect::<Vec<_>>();
    if header.len() != 6 || header[0] != GENE_RESIDENCY_SCHEMA_VERSION || header[1] != genome.id {
        return Err(invalid_genome_state("gene residency header mismatch"));
    }
    let generation = header[2]
        .parse::<u64>()
        .map_err(|_| invalid_genome_state("gene residency generation is invalid"))?;
    let step = header[3]
        .parse::<u64>()
        .map_err(|_| invalid_genome_state("gene residency step is invalid"))?;
    let last_observation_sequence = header[4]
        .parse::<u64>()
        .map_err(|_| invalid_genome_state("gene residency observation sequence is invalid"))?;
    if header[5].trim().is_empty() {
        return Err(invalid_genome_state("gene residency reason missing"));
    }
    let mut records = Vec::new();
    for line in lines {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 8 || fields[0] != "gene" {
            return Err(invalid_genome_state("gene residency record is malformed"));
        }
        records.push(GeneUsageRecord {
            gene_id: fields[1].to_owned(),
            residency: parse_residency_state(fields[2])?,
            opportunities: parse_u64_field(fields[3])?,
            hits: parse_u64_field(fields[4])?,
            failures: parse_u64_field(fields[5])?,
            last_used_step: parse_u64_field(fields[6])?,
            consumed_evidence_digest: fields[7].to_owned(),
        });
    }
    let residency = GenomeGeneResidency {
        step,
        records,
        last_observation_sequence,
        last_transition_reason: header[5].to_owned(),
    };
    validate_gene_residency(&residency, genome)?;
    Ok((generation, residency))
}

fn validate_gene_residency(
    residency: &GenomeGeneResidency,
    genome: &ReasoningGenome,
) -> io::Result<()> {
    if genome.genes.len() > GENOME_PERSISTED_GENE_CAPACITY {
        return Err(invalid_genome_state(
            "genome persisted gene capacity exceeded",
        ));
    }
    let retired_count = residency
        .records
        .iter()
        .filter(|record| record.residency == MemoryResidencyState::Retired)
        .count();
    if retired_count > GENOME_PERSISTED_GENE_CAPACITY
        || residency.records.len()
            > genome
                .genes
                .len()
                .saturating_add(GENOME_PERSISTED_GENE_CAPACITY)
    {
        return Err(invalid_genome_state(
            "gene residency sidecar exceeds capacity",
        ));
    }
    let resident_count = residency
        .records
        .iter()
        .filter(|record| {
            matches!(
                record.residency,
                MemoryResidencyState::Hot | MemoryResidencyState::Warm
            )
        })
        .count();
    if resident_count > crate::adaptive_state::GENOME_EXPRESSED_GENE_CAPACITY {
        return Err(invalid_genome_state(
            "gene residency expressed capacity exceeded",
        ));
    }
    if residency.records.iter().any(|record| {
        record.gene_id.trim().is_empty()
            || invalid_sidecar_field(&record.gene_id)
            || invalid_sidecar_field(&record.consumed_evidence_digest)
            || record.hits.saturating_add(record.failures) > record.opportunities
            || record.last_used_step > residency.step
            || (!record.consumed_evidence_digest.is_empty()
                && !record
                    .consumed_evidence_digest
                    .starts_with("redaction-digest:"))
    }) {
        return Err(invalid_genome_state("gene residency record is invalid"));
    }
    if residency.records.iter().enumerate().any(|(index, record)| {
        residency.records[index + 1..]
            .iter()
            .any(|other| other.gene_id == record.gene_id)
    }) {
        return Err(invalid_genome_state("gene residency record is duplicated"));
    }
    if genome.genes.iter().any(|gene| {
        let records = residency
            .records
            .iter()
            .filter(|record| record.gene_id == gene.id)
            .collect::<Vec<_>>();
        records.len() != 1 || records[0].residency == MemoryResidencyState::Retired
    }) {
        return Err(invalid_genome_state(
            "gene residency payload mapping mismatch",
        ));
    }
    if residency.records.iter().any(|record| {
        record.residency != MemoryResidencyState::Retired
            && genome.genes.iter().all(|gene| gene.id != record.gene_id)
    }) {
        return Err(invalid_genome_state("gene residency orphan record"));
    }
    Ok(())
}

fn parse_residency_state(value: &str) -> io::Result<MemoryResidencyState> {
    match value {
        "hot" => Ok(MemoryResidencyState::Hot),
        "warm" => Ok(MemoryResidencyState::Warm),
        "cold" => Ok(MemoryResidencyState::Cold),
        "quarantined" => Ok(MemoryResidencyState::Quarantined),
        "retired" => Ok(MemoryResidencyState::Retired),
        _ => Err(invalid_genome_state("gene residency state is invalid")),
    }
}

fn parse_u64_field(value: &str) -> io::Result<u64> {
    value
        .parse::<u64>()
        .map_err(|_| invalid_genome_state("gene residency counter is invalid"))
}

fn invalid_sidecar_field(value: &str) -> bool {
    value.contains(['\t', '\r', '\n'])
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
