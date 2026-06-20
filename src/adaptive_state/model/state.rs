use crate::hierarchy::HierarchyState;
use crate::kv_cache::{MemoryCompactionPolicy, MemoryRetentionPolicy};
use crate::router::RouterState;
use crate::tiered_cache::TieredCachePlan;

use super::EvolutionLedger;

#[derive(Debug, Clone)]
pub struct AdaptiveState {
    pub router: RouterState,
    pub hierarchy: HierarchyState,
    pub tier_plan: TieredCachePlan,
    pub memory_retention_policy: MemoryRetentionPolicy,
    pub memory_compaction_policy: MemoryCompactionPolicy,
    pub evolution_ledger: EvolutionLedger,
}
