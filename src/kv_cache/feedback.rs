use super::cache::KvFusionCache;
use super::model::{MemoryUpdateAction, MemoryUpdateReport};

impl KvFusionCache {
    pub fn reinforce(&mut self, id: u64, amount: f32) -> MemoryUpdateReport {
        let amount = amount.clamp(0.0, 1.0);
        if let Some(index) = self.entries.iter().position(|entry| entry.id == id) {
            let now = self.tick();
            let entry = &mut self.entries[index];
            let strength_before = entry.strength;
            entry.strength = (entry.strength + amount * 0.18).clamp(0.01, 3.0);
            entry.hits += 1;
            entry.last_access = now;
            return MemoryUpdateReport::applied(
                id,
                MemoryUpdateAction::Reinforce,
                amount,
                strength_before,
                entry.strength,
                false,
            );
        }

        MemoryUpdateReport::missing(id, MemoryUpdateAction::Reinforce, amount)
    }

    pub fn penalize(&mut self, id: u64, amount: f32) -> MemoryUpdateReport {
        let amount = amount.clamp(0.0, 1.0);
        let mut report = MemoryUpdateReport::missing(id, MemoryUpdateAction::Penalize, amount);
        if let Some(index) = self.entries.iter().position(|entry| entry.id == id) {
            let now = self.tick();
            let entry = &mut self.entries[index];
            let strength_before = entry.strength;
            entry.strength = (entry.strength - amount * 0.22).clamp(0.0, 3.0);
            entry.failures += 1;
            entry.last_access = now;
            report = MemoryUpdateReport::applied(
                id,
                MemoryUpdateAction::Penalize,
                amount,
                strength_before,
                entry.strength,
                false,
            );
        }
        self.entries
            .retain(|entry| entry.id != id || entry.strength > 0.03 || entry.hits > entry.failures);
        if report.was_applied() && self.entries.iter().all(|entry| entry.id != id) {
            report.removed = true;
        }
        report
    }
}
