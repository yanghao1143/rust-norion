use crate::kv_cache::{MemoryUpdateAction, MemoryUpdateReport};

use super::ExperienceReplayReport;

impl ExperienceReplayReport {
    pub fn record_memory_update(&mut self, update: MemoryUpdateReport) {
        self.touched_memories += 1;
        match update.action {
            MemoryUpdateAction::Reinforce => self.memory_reinforcements += 1,
            MemoryUpdateAction::Penalize => self.memory_penalties += 1,
        }
        if update.was_applied() {
            self.applied_memory_updates += 1;
        } else {
            self.missing_memory_updates += 1;
        }
        if update.removed {
            self.removed_memory_updates += 1;
        }
        self.memory_strength_delta += update.strength_delta.abs();
        self.memory_update_reports.push(update);
    }
}
