use super::cache::KvFusionCache;
use super::model::{MemoryRetentionPolicy, RetentionReport};

impl KvFusionCache {
    pub fn apply_retention(&mut self, policy: MemoryRetentionPolicy) -> RetentionReport {
        self.apply_retention_with_protected(policy, &[])
    }

    pub fn apply_retention_with_protected(
        &mut self,
        policy: MemoryRetentionPolicy,
        protected_ids: &[u64],
    ) -> RetentionReport {
        let before = self.entries.len();
        let now = self.tick();
        let stale_after = policy.stale_after.max(1);
        let decay_rate = policy.decay_rate.clamp(0.0, 0.95);
        let mut decayed = 0;

        for entry in &mut self.entries {
            let idle = now.saturating_sub(entry.last_access);
            if idle <= policy.stale_after {
                continue;
            }

            let periods = (idle - policy.stale_after) as f32 / stale_after as f32;
            let decay = (decay_rate * periods.max(1.0)).clamp(0.0, 0.95);
            let before_strength = entry.strength;
            entry.strength = (entry.strength * (1.0 - decay)).clamp(0.0, 3.0);
            if entry.strength < before_strength {
                decayed += 1;
            }
        }

        let mut removed = Vec::new();
        self.entries.retain(|entry| {
            if protected_ids.contains(&entry.id) {
                return true;
            }
            let idle = now.saturating_sub(entry.last_access);
            let weak_and_stale = entry.strength <= policy.remove_below_strength
                && idle > policy.stale_after
                && entry.failures >= entry.hits;
            let repeatedly_failed =
                entry.failures >= policy.remove_after_failures && entry.hits == 0;
            let remove = weak_and_stale || repeatedly_failed;
            if remove {
                removed.push(entry.id);
            }
            !remove
        });

        RetentionReport {
            before,
            after: self.entries.len(),
            decayed,
            removed,
        }
    }
}
