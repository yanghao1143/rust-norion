use std::io;
use std::path::Path;

use crate::disk_kv::DiskKvStore;
use crate::hierarchy::{HierarchyState, HierarchyWeights};
use crate::router::{ProfileObservations, ProfileThresholds, RouterState};
use crate::tiered_cache::{MemoryPlacement, MemoryTier, TieredCachePlan};

#[derive(Debug, Clone)]
pub struct AdaptiveState {
    pub router: RouterState,
    pub hierarchy: HierarchyState,
    pub tier_plan: TieredCachePlan,
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
            format!(
                "{:.6}\t{:.6}\t{:.6}",
                self.hierarchy.current.global,
                self.hierarchy.current.local,
                self.hierarchy.current.convolution
            )
            .as_bytes(),
        )?;
        store.put(
            "adaptive/tier_plan",
            serialize_tier_plan(&self.tier_plan).as_bytes(),
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

        Ok(Some(Self {
            router,
            hierarchy,
            tier_plan,
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
    if fields.len() != 3 {
        return None;
    }

    Some(HierarchyState {
        current: HierarchyWeights::new(
            fields[0].parse::<f32>().ok()?,
            fields[1].parse::<f32>().ok()?,
            fields[2].parse::<f32>().ok()?,
        ),
    })
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
            },
            tier_plan: TieredCachePlan::new(vec![MemoryPlacement {
                id: 7,
                tier: MemoryTier::WarmRam,
                score: 0.42,
                reason: "warm\tstate".to_owned(),
            }]),
        };

        state.save_to_disk_kv(&path).unwrap();
        let loaded = AdaptiveState::load_from_disk_kv(&path).unwrap().unwrap();

        assert!((loaded.router.threshold - 0.61).abs() < 0.0001);
        assert_eq!(loaded.router.observations, 17);
        assert!((loaded.router.profile_thresholds.coding - 0.49).abs() < 0.0001);
        assert_eq!(loaded.router.profile_observations.writing, 3);
        assert!((loaded.hierarchy.current.local - 0.6).abs() < 0.0001);
        let placement = loaded.tier_plan.placement_for(7).unwrap();
        assert_eq!(placement.tier, MemoryTier::WarmRam);
        assert_eq!(placement.reason, "warm\tstate");
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
