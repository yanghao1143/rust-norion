use std::io;
use std::path::Path;

use crate::disk_kv::DiskKvStore;
use crate::experience_replay::ExperienceReplayReport;
use crate::hierarchy::{
    HierarchyState, HierarchyWeights, ProfileHierarchyObservations, ProfileHierarchyWeights,
};
use crate::kv_cache::{MemoryCompactionPolicy, MemoryRetentionPolicy};
use crate::router::{ProfileObservations, ProfileThresholds, RouterState};
use crate::tiered_cache::{MemoryPlacement, MemoryTier, TieredCachePlan};

#[derive(Debug, Clone)]
pub struct AdaptiveState {
    pub router: RouterState,
    pub hierarchy: HierarchyState,
    pub tier_plan: TieredCachePlan,
    pub memory_retention_policy: MemoryRetentionPolicy,
    pub memory_compaction_policy: MemoryCompactionPolicy,
    pub evolution_ledger: EvolutionLedger,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EvolutionLedger {
    pub replay_runs: u64,
    pub replay_items: u64,
    pub router_threshold_mutations: u64,
    pub hierarchy_weight_mutations: u64,
    pub router_threshold_delta: f32,
    pub hierarchy_weight_delta: f32,
    pub memory_reinforcements: u64,
    pub memory_penalties: u64,
    pub recursive_replay_items: u64,
    pub recursive_runtime_calls: u64,
    pub drift_rollbacks: u64,
    pub rollback_router_threshold_delta: f32,
    pub rollback_hierarchy_weight_delta: f32,
}

impl EvolutionLedger {
    pub fn record_replay(&mut self, report: &ExperienceReplayReport) {
        if report.applied == 0 {
            return;
        }

        self.replay_runs = self.replay_runs.saturating_add(1);
        self.replay_items = self.replay_items.saturating_add(report.applied as u64);
        self.router_threshold_mutations = self
            .router_threshold_mutations
            .saturating_add(report.router_threshold_mutations as u64);
        self.hierarchy_weight_mutations = self
            .hierarchy_weight_mutations
            .saturating_add(report.hierarchy_weight_mutations as u64);
        self.router_threshold_delta += report.router_threshold_delta;
        self.hierarchy_weight_delta += report.hierarchy_weight_delta;
        self.memory_reinforcements = self
            .memory_reinforcements
            .saturating_add(report.memory_reinforcements as u64);
        self.memory_penalties = self
            .memory_penalties
            .saturating_add(report.memory_penalties as u64);
        self.recursive_replay_items = self
            .recursive_replay_items
            .saturating_add(report.recursive_runtime_items as u64);
        self.recursive_runtime_calls = self
            .recursive_runtime_calls
            .saturating_add(report.recursive_runtime_calls as u64);
    }

    pub fn memory_updates(self) -> u64 {
        self.memory_reinforcements
            .saturating_add(self.memory_penalties)
    }

    pub fn record_drift_rollback(
        &mut self,
        router_threshold_delta: f32,
        hierarchy_weight_delta: f32,
    ) {
        self.drift_rollbacks = self.drift_rollbacks.saturating_add(1);
        self.rollback_router_threshold_delta += router_threshold_delta.max(0.0);
        self.rollback_hierarchy_weight_delta += hierarchy_weight_delta.max(0.0);
    }

    pub fn summary_line(self) -> String {
        format!(
            "evolution: replay_runs={} replay_items={} router_threshold_mutations={} hierarchy_weight_mutations={} router_threshold_delta={:.6} hierarchy_weight_delta={:.6} memory_updates={} recursive_replay_items={} recursive_runtime_calls={} drift_rollbacks={} rollback_router_threshold_delta={:.6} rollback_hierarchy_weight_delta={:.6}",
            self.replay_runs,
            self.replay_items,
            self.router_threshold_mutations,
            self.hierarchy_weight_mutations,
            self.router_threshold_delta,
            self.hierarchy_weight_delta,
            self.memory_updates(),
            self.recursive_replay_items,
            self.recursive_runtime_calls,
            self.drift_rollbacks,
            self.rollback_router_threshold_delta,
            self.rollback_hierarchy_weight_delta
        )
    }
}

impl AdaptiveState {
    pub fn save_to_disk_kv(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let mut store = DiskKvStore::open(path)?;
        store.put(
            "adaptive/router",
            format!(
                "{:.6}\t{}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{}\t{}\t{}\t{}",
                self.router.threshold,
                self.router.observations,
                self.router.profile_thresholds.general,
                self.router.profile_thresholds.coding,
                self.router.profile_thresholds.writing,
                self.router.profile_thresholds.long_document,
                self.router.profile_observations.general,
                self.router.profile_observations.coding,
                self.router.profile_observations.writing,
                self.router.profile_observations.long_document
            )
            .as_bytes(),
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

        let tier_plan = if let Some(tier_bytes) = store.get("adaptive/tier_plan")? {
            parse_tier_plan(&String::from_utf8_lossy(&tier_bytes))
        } else {
            TieredCachePlan::default()
        };
        let memory_retention_policy =
            if let Some(retention_bytes) = store.get("adaptive/memory_retention")? {
                parse_memory_retention_policy(&String::from_utf8_lossy(&retention_bytes))
                    .unwrap_or_default()
            } else {
                MemoryRetentionPolicy::default()
            };
        let memory_compaction_policy =
            if let Some(compaction_bytes) = store.get("adaptive/memory_compaction")? {
                parse_memory_compaction_policy(&String::from_utf8_lossy(&compaction_bytes))
                    .unwrap_or_default()
            } else {
                MemoryCompactionPolicy::default()
            };
        let evolution_ledger = if let Some(ledger_bytes) = store.get("adaptive/evolution_ledger")? {
            parse_evolution_ledger(&String::from_utf8_lossy(&ledger_bytes)).unwrap_or_default()
        } else {
            EvolutionLedger::default()
        };

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

fn parse_router_state(value: &str) -> Option<RouterState> {
    let fields = value.split('\t').collect::<Vec<_>>();
    if fields.len() != 2 && fields.len() != 10 {
        return None;
    }

    let threshold = fields[0].parse::<f32>().ok()?;
    let observations = fields[1].parse::<u64>().ok()?;
    let profile_thresholds = if fields.len() == 10 {
        ProfileThresholds {
            general: fields[2].parse::<f32>().ok()?,
            coding: fields[3].parse::<f32>().ok()?,
            writing: fields[4].parse::<f32>().ok()?,
            long_document: fields[5].parse::<f32>().ok()?,
        }
    } else {
        ProfileThresholds::from_single(threshold)
    };
    let profile_observations = if fields.len() == 10 {
        ProfileObservations {
            general: fields[6].parse::<u64>().ok()?,
            coding: fields[7].parse::<u64>().ok()?,
            writing: fields[8].parse::<u64>().ok()?,
            long_document: fields[9].parse::<u64>().ok()?,
        }
    } else {
        ProfileObservations::from_single(observations)
    };

    Some(RouterState {
        threshold,
        observations,
        profile_thresholds,
        profile_observations,
    })
}

fn parse_hierarchy_state(value: &str) -> Option<HierarchyState> {
    let fields = value.split('\t').collect::<Vec<_>>();
    if fields.len() != 3 && fields.len() != 19 {
        return None;
    }

    let current = HierarchyWeights::new(
        fields[0].parse::<f32>().ok()?,
        fields[1].parse::<f32>().ok()?,
        fields[2].parse::<f32>().ok()?,
    );
    let profile_weights = if fields.len() == 19 {
        ProfileHierarchyWeights {
            general: parse_hierarchy_weights(&fields[3..6])?,
            coding: parse_hierarchy_weights(&fields[6..9])?,
            writing: parse_hierarchy_weights(&fields[9..12])?,
            long_document: parse_hierarchy_weights(&fields[12..15])?,
        }
    } else {
        ProfileHierarchyWeights::from_single(current)
    };
    let profile_observations = if fields.len() == 19 {
        ProfileHierarchyObservations {
            general: fields[15].parse::<u64>().ok()?,
            coding: fields[16].parse::<u64>().ok()?,
            writing: fields[17].parse::<u64>().ok()?,
            long_document: fields[18].parse::<u64>().ok()?,
        }
    } else {
        ProfileHierarchyObservations::default()
    };

    Some(HierarchyState {
        current,
        profile_weights,
        profile_observations,
    })
}

fn serialize_hierarchy_state(state: HierarchyState) -> String {
    format!(
        "{:.6}\t{:.6}\t{:.6}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        state.current.global,
        state.current.local,
        state.current.convolution,
        serialize_hierarchy_weights(state.profile_weights.general),
        serialize_hierarchy_weights(state.profile_weights.coding),
        serialize_hierarchy_weights(state.profile_weights.writing),
        serialize_hierarchy_weights(state.profile_weights.long_document),
        state.profile_observations.general,
        state.profile_observations.coding,
        state.profile_observations.writing,
        state.profile_observations.long_document
    )
}

fn serialize_hierarchy_weights(weights: HierarchyWeights) -> String {
    format!(
        "{:.6}\t{:.6}\t{:.6}",
        weights.global, weights.local, weights.convolution
    )
}

fn parse_hierarchy_weights(fields: &[&str]) -> Option<HierarchyWeights> {
    if fields.len() != 3 {
        return None;
    }

    Some(HierarchyWeights::new(
        fields[0].parse::<f32>().ok()?,
        fields[1].parse::<f32>().ok()?,
        fields[2].parse::<f32>().ok()?,
    ))
}

fn serialize_tier_plan(plan: &TieredCachePlan) -> String {
    plan.placements()
        .iter()
        .map(|placement| {
            format!(
                "{}\t{}\t{:.6}\t{}",
                placement.id,
                placement.tier.as_str(),
                placement.score,
                escape_field(&placement.reason)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_tier_plan(value: &str) -> TieredCachePlan {
    let placements = value
        .lines()
        .filter_map(parse_memory_placement)
        .collect::<Vec<_>>();
    TieredCachePlan::new(placements)
}

fn serialize_memory_retention_policy(policy: MemoryRetentionPolicy) -> String {
    format!(
        "{}\t{:.6}\t{:.6}\t{}",
        policy.stale_after,
        policy.decay_rate,
        policy.remove_below_strength,
        policy.remove_after_failures
    )
}

fn parse_memory_retention_policy(value: &str) -> Option<MemoryRetentionPolicy> {
    let fields = value.split('\t').collect::<Vec<_>>();
    if fields.len() != 4 {
        return None;
    }

    Some(MemoryRetentionPolicy {
        stale_after: fields[0].parse::<u64>().ok()?.max(1),
        decay_rate: fields[1].parse::<f32>().ok()?.clamp(0.0, 0.95),
        remove_below_strength: fields[2].parse::<f32>().ok()?.clamp(0.0, 3.0),
        remove_after_failures: fields[3].parse::<u64>().ok()?.max(1),
    })
}

fn serialize_memory_compaction_policy(policy: &MemoryCompactionPolicy) -> String {
    format!(
        "{:.6}\t{}\t{}",
        policy.similarity_threshold, policy.max_candidates, policy.max_merges
    )
}

fn parse_memory_compaction_policy(value: &str) -> Option<MemoryCompactionPolicy> {
    let fields = value.split('\t').collect::<Vec<_>>();
    if fields.len() != 3 {
        return None;
    }

    Some(MemoryCompactionPolicy {
        similarity_threshold: fields[0].parse::<f32>().ok()?.clamp(0.10, 0.999),
        max_candidates: fields[1].parse::<usize>().ok()?.max(2),
        max_merges: fields[2].parse::<usize>().ok()?,
    })
}

fn serialize_evolution_ledger(ledger: EvolutionLedger) -> String {
    format!(
        "{}\t{}\t{}\t{}\t{:.6}\t{:.6}\t{}\t{}\t{}\t{}\t{}\t{:.6}\t{:.6}",
        ledger.replay_runs,
        ledger.replay_items,
        ledger.router_threshold_mutations,
        ledger.hierarchy_weight_mutations,
        ledger.router_threshold_delta,
        ledger.hierarchy_weight_delta,
        ledger.memory_reinforcements,
        ledger.memory_penalties,
        ledger.recursive_replay_items,
        ledger.recursive_runtime_calls,
        ledger.drift_rollbacks,
        ledger.rollback_router_threshold_delta,
        ledger.rollback_hierarchy_weight_delta
    )
}

fn parse_evolution_ledger(value: &str) -> Option<EvolutionLedger> {
    let fields = value.split('\t').collect::<Vec<_>>();
    if fields.len() != 10 && fields.len() != 13 {
        return None;
    }

    Some(EvolutionLedger {
        replay_runs: fields[0].parse::<u64>().ok()?,
        replay_items: fields[1].parse::<u64>().ok()?,
        router_threshold_mutations: fields[2].parse::<u64>().ok()?,
        hierarchy_weight_mutations: fields[3].parse::<u64>().ok()?,
        router_threshold_delta: fields[4].parse::<f32>().ok()?.max(0.0),
        hierarchy_weight_delta: fields[5].parse::<f32>().ok()?.max(0.0),
        memory_reinforcements: fields[6].parse::<u64>().ok()?,
        memory_penalties: fields[7].parse::<u64>().ok()?,
        recursive_replay_items: fields[8].parse::<u64>().ok()?,
        recursive_runtime_calls: fields[9].parse::<u64>().ok()?,
        drift_rollbacks: fields
            .get(10)
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(0),
        rollback_router_threshold_delta: fields
            .get(11)
            .and_then(|value| value.parse::<f32>().ok())
            .unwrap_or(0.0)
            .max(0.0),
        rollback_hierarchy_weight_delta: fields
            .get(12)
            .and_then(|value| value.parse::<f32>().ok())
            .unwrap_or(0.0)
            .max(0.0),
    })
}

fn parse_memory_placement(value: &str) -> Option<MemoryPlacement> {
    let fields = value.split('\t').collect::<Vec<_>>();
    if fields.len() != 4 {
        return None;
    }

    Some(MemoryPlacement {
        id: fields[0].parse::<u64>().ok()?,
        tier: fields[1].parse::<MemoryTier>().ok()?,
        score: fields[2].parse::<f32>().ok()?,
        reason: unescape_field(fields[3]),
    })
}

fn escape_field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn unescape_field(value: &str) -> String {
    let mut out = String::new();
    let mut chars = value.chars();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }

        match chars.next() {
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn adaptive_state_roundtrips_through_disk_kv() {
        let path = temp_path("adaptive-state");
        let state = AdaptiveState {
            router: RouterState {
                threshold: 0.61,
                observations: 17,
                profile_thresholds: ProfileThresholds {
                    general: 0.61,
                    coding: 0.49,
                    writing: 0.66,
                    long_document: 0.42,
                },
                profile_observations: ProfileObservations {
                    general: 8,
                    coding: 5,
                    writing: 3,
                    long_document: 1,
                },
            },
            hierarchy: HierarchyState {
                current: HierarchyWeights::new(0.2, 0.6, 0.2),
                profile_weights: ProfileHierarchyWeights {
                    general: HierarchyWeights::new(0.36, 0.42, 0.22),
                    coding: HierarchyWeights::new(0.18, 0.68, 0.14),
                    writing: HierarchyWeights::new(0.60, 0.26, 0.14),
                    long_document: HierarchyWeights::new(0.24, 0.18, 0.58),
                },
                profile_observations: ProfileHierarchyObservations {
                    general: 2,
                    coding: 7,
                    writing: 5,
                    long_document: 3,
                },
            },
            tier_plan: TieredCachePlan::new(vec![MemoryPlacement {
                id: 7,
                tier: MemoryTier::WarmRam,
                score: 0.42,
                reason: "warm\tstate".to_owned(),
            }]),
            memory_retention_policy: MemoryRetentionPolicy {
                stale_after: 11,
                decay_rate: 0.12,
                remove_below_strength: 0.08,
                remove_after_failures: 7,
            },
            memory_compaction_policy: MemoryCompactionPolicy {
                similarity_threshold: 0.91,
                max_candidates: 64,
                max_merges: 4,
            },
            evolution_ledger: EvolutionLedger {
                replay_runs: 3,
                replay_items: 9,
                router_threshold_mutations: 5,
                hierarchy_weight_mutations: 7,
                router_threshold_delta: 0.42,
                hierarchy_weight_delta: 0.21,
                memory_reinforcements: 4,
                memory_penalties: 2,
                recursive_replay_items: 1,
                recursive_runtime_calls: 8,
                drift_rollbacks: 2,
                rollback_router_threshold_delta: 0.11,
                rollback_hierarchy_weight_delta: 0.09,
            },
        };

        state.save_to_disk_kv(&path).unwrap();
        let loaded = AdaptiveState::load_from_disk_kv(&path).unwrap().unwrap();

        assert!((loaded.router.threshold - 0.61).abs() < 0.0001);
        assert_eq!(loaded.router.observations, 17);
        assert!((loaded.router.profile_thresholds.coding - 0.49).abs() < 0.0001);
        assert_eq!(loaded.router.profile_observations.writing, 3);
        assert!((loaded.hierarchy.current.local - 0.6).abs() < 0.0001);
        assert!((loaded.hierarchy.profile_weights.coding.local - 0.68).abs() < 0.0001);
        assert_eq!(loaded.hierarchy.profile_observations.long_document, 3);
        let placement = loaded.tier_plan.placement_for(7).unwrap();
        assert_eq!(placement.tier, MemoryTier::WarmRam);
        assert_eq!(placement.reason, "warm\tstate");
        assert_eq!(loaded.memory_retention_policy.stale_after, 11);
        assert!((loaded.memory_retention_policy.decay_rate - 0.12).abs() < 0.0001);
        assert!((loaded.memory_retention_policy.remove_below_strength - 0.08).abs() < 0.0001);
        assert_eq!(loaded.memory_retention_policy.remove_after_failures, 7);
        assert!((loaded.memory_compaction_policy.similarity_threshold - 0.91).abs() < 0.0001);
        assert_eq!(loaded.memory_compaction_policy.max_candidates, 64);
        assert_eq!(loaded.memory_compaction_policy.max_merges, 4);
        assert_eq!(loaded.evolution_ledger.replay_runs, 3);
        assert_eq!(loaded.evolution_ledger.replay_items, 9);
        assert_eq!(loaded.evolution_ledger.router_threshold_mutations, 5);
        assert_eq!(loaded.evolution_ledger.hierarchy_weight_mutations, 7);
        assert!((loaded.evolution_ledger.router_threshold_delta - 0.42).abs() < 0.0001);
        assert!((loaded.evolution_ledger.hierarchy_weight_delta - 0.21).abs() < 0.0001);
        assert_eq!(loaded.evolution_ledger.memory_updates(), 6);
        assert_eq!(loaded.evolution_ledger.recursive_replay_items, 1);
        assert_eq!(loaded.evolution_ledger.recursive_runtime_calls, 8);
        assert_eq!(loaded.evolution_ledger.drift_rollbacks, 2);
        assert!((loaded.evolution_ledger.rollback_router_threshold_delta - 0.11).abs() < 0.0001);
        assert!((loaded.evolution_ledger.rollback_hierarchy_weight_delta - 0.09).abs() < 0.0001);
        cleanup(path);
    }

    #[test]
    fn evolution_ledger_loads_legacy_without_rollback_fields() {
        let legacy = "3\t9\t5\t7\t0.420000\t0.210000\t4\t2\t1\t8";
        let ledger = parse_evolution_ledger(legacy).unwrap();

        assert_eq!(ledger.replay_runs, 3);
        assert_eq!(ledger.memory_updates(), 6);
        assert_eq!(ledger.recursive_runtime_calls, 8);
        assert_eq!(ledger.drift_rollbacks, 0);
        assert_eq!(ledger.rollback_router_threshold_delta, 0.0);
        assert_eq!(ledger.rollback_hierarchy_weight_delta, 0.0);
    }

    #[test]
    fn adaptive_state_loads_legacy_files_without_memory_policies() {
        let path = temp_path("adaptive-state-legacy");
        {
            let mut store = DiskKvStore::open(&path).unwrap();
            store.put("adaptive/router", b"0.610000\t17").unwrap();
            store
                .put("adaptive/hierarchy", b"0.200000\t0.600000\t0.200000")
                .unwrap();
            store.compact().unwrap();
        }

        let loaded = AdaptiveState::load_from_disk_kv(&path).unwrap().unwrap();

        assert!((loaded.router.threshold - 0.61).abs() < 0.0001);
        assert_eq!(loaded.router.observations, 17);
        assert!((loaded.hierarchy.current.local - 0.6).abs() < 0.0001);
        assert_eq!(
            loaded.memory_retention_policy.stale_after,
            MemoryRetentionPolicy::default().stale_after
        );
        assert!(
            (loaded.memory_compaction_policy.similarity_threshold
                - MemoryCompactionPolicy::default().similarity_threshold)
                .abs()
                < 0.0001
        );
        assert_eq!(loaded.evolution_ledger, EvolutionLedger::default());
        cleanup(path);
    }

    fn temp_path(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "rust-norion-{label}-{}-{nanos}.ndkv",
            std::process::id()
        ))
    }

    fn cleanup(path: std::path::PathBuf) {
        let _ = fs::remove_file(path);
    }
}
