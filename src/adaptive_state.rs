use std::io;
use std::path::Path;

use crate::disk_kv::DiskKvStore;
use crate::hierarchy::{HierarchyState, HierarchyWeights};
use crate::router::RouterState;

#[derive(Debug, Clone, Copy)]
pub struct AdaptiveState {
    pub router: RouterState,
    pub hierarchy: HierarchyState,
}

impl AdaptiveState {
    pub fn save_to_disk_kv(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let mut store = DiskKvStore::open(path)?;
        store.put(
            "adaptive/router",
            format!("{:.6}\t{}", self.router.threshold, self.router.observations).as_bytes(),
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

        Ok(Some(Self { router, hierarchy }))
    }
}

fn parse_router_state(value: &str) -> Option<RouterState> {
    let fields = value.split('\t').collect::<Vec<_>>();
    if fields.len() != 2 {
        return None;
    }

    Some(RouterState {
        threshold: fields[0].parse::<f32>().ok()?,
        observations: fields[1].parse::<u64>().ok()?,
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
            },
            hierarchy: HierarchyState {
                current: HierarchyWeights::new(0.2, 0.6, 0.2),
            },
        };

        state.save_to_disk_kv(&path).unwrap();
        let loaded = AdaptiveState::load_from_disk_kv(&path).unwrap().unwrap();

        assert!((loaded.router.threshold - 0.61).abs() < 0.0001);
        assert_eq!(loaded.router.observations, 17);
        assert!((loaded.hierarchy.current.local - 0.6).abs() < 0.0001);
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
