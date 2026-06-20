use std::io;
use std::path::Path;

use crate::adaptive_state::AdaptiveState;
use crate::experience::ExperienceStore;
use crate::hardware::HardwareSnapshot;
use crate::kv_cache::{KvFusionCache, MemoryCompactionPolicy, MemoryRetentionPolicy};

use super::NoironEngine;

impl NoironEngine {
    pub fn load_memory(path: impl AsRef<Path>) -> io::Result<Self> {
        Ok(Self::with_cache(KvFusionCache::load_persistent(path)?))
    }

    pub fn load_state(
        memory_path: impl AsRef<Path>,
        experience_path: impl AsRef<Path>,
    ) -> io::Result<Self> {
        let mut engine = Self::load_memory(memory_path)?;
        engine.experience = ExperienceStore::load_from_disk_kv(experience_path)?;
        Ok(engine)
    }

    pub fn load_full_state(
        memory_path: impl AsRef<Path>,
        experience_path: impl AsRef<Path>,
        adaptive_path: impl AsRef<Path>,
    ) -> io::Result<Self> {
        let mut engine = Self::load_state(memory_path, experience_path)?;
        if let Some(state) = AdaptiveState::load_from_disk_kv(adaptive_path)? {
            engine.restore_adaptive_state(state);
        }
        Ok(engine)
    }

    pub fn save_memory(&self, path: impl AsRef<Path>) -> io::Result<()> {
        self.cache.save_persistent(path)
    }

    pub fn save_experience(&self, path: impl AsRef<Path>) -> io::Result<()> {
        self.experience.save_to_disk_kv(path)
    }

    pub fn adaptive_state(&self) -> AdaptiveState {
        AdaptiveState {
            router: self.router.state(),
            hierarchy: self.hierarchy.state(),
            tier_plan: self.last_tier_plan.clone(),
            memory_retention_policy: self.memory_retention_policy,
            memory_compaction_policy: self.memory_compaction_policy.clone(),
            evolution_ledger: self.evolution_ledger,
        }
    }

    pub fn restore_adaptive_state(&mut self, state: AdaptiveState) {
        self.router.restore_state(state.router);
        self.hierarchy.restore_state(state.hierarchy);
        self.last_tier_plan = state.tier_plan;
        self.memory_retention_policy = state.memory_retention_policy;
        self.memory_compaction_policy = state.memory_compaction_policy;
        self.evolution_ledger = state.evolution_ledger;
    }

    pub fn save_adaptive_state(&self, path: impl AsRef<Path>) -> io::Result<()> {
        self.adaptive_state().save_to_disk_kv(path)
    }

    pub fn save_full_state(
        &self,
        memory_path: impl AsRef<Path>,
        experience_path: impl AsRef<Path>,
        adaptive_path: impl AsRef<Path>,
    ) -> io::Result<()> {
        self.save_memory(memory_path)?;
        self.save_experience(experience_path)?;
        self.save_adaptive_state(adaptive_path)
    }

    pub fn set_hardware_snapshot(&mut self, snapshot: HardwareSnapshot) {
        self.hardware_snapshot = snapshot;
    }

    pub fn set_auto_replay_limit(&mut self, limit: usize) {
        self.auto_replay_limit = limit;
    }

    pub fn set_memory_retention_policy(&mut self, policy: MemoryRetentionPolicy) {
        self.memory_retention_policy = policy;
    }

    pub fn set_memory_compaction_policy(&mut self, policy: MemoryCompactionPolicy) {
        self.memory_compaction_policy = policy;
    }
}
