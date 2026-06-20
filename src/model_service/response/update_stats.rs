use rust_norion::MemoryUpdateReport;

pub(super) fn memory_update_applied_count(updates: &[MemoryUpdateReport]) -> usize {
    updates.iter().filter(|update| update.was_applied()).count()
}

pub(super) fn memory_update_removed_count(updates: &[MemoryUpdateReport]) -> usize {
    updates.iter().filter(|update| update.removed).count()
}

pub(super) fn memory_update_missing_count(updates: &[MemoryUpdateReport]) -> usize {
    updates
        .len()
        .saturating_sub(memory_update_applied_count(updates))
}

pub(super) fn memory_update_strength_delta(updates: &[MemoryUpdateReport]) -> f32 {
    updates
        .iter()
        .map(|update| update.strength_delta.abs())
        .sum::<f32>()
}
