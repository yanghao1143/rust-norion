use crate::adaptive_state::{EvolutionLedger, GenomeRuntimeState};
use crate::agent_team::AgentTeamPlanner;
use crate::drift::DriftGuard;
use crate::experience::ExperienceStore;
use crate::experience_replay::ExperienceReplayPlanner;
use crate::gist_memory::GistGenerator;
use crate::hardware::{HardwareAllocator, HardwareSnapshot};
use crate::hierarchy::HierarchyController;
use crate::homeostasis::HomeostaticSetpoints;
use crate::infini_memory::InfiniMemoryPlanner;
use crate::kv_cache::{KvFusionCache, MemoryCompactionPolicy, MemoryRetentionPolicy};
use crate::process_reward::ProcessRewarder;
use crate::recursive_scheduler::RecursiveScheduler;
use crate::reflection::Reflector;
use crate::router::NoironRouter;
use crate::tiered_cache::{TieredCachePlan, TieredCacheScheduler};
use crate::token_stream::TokenStreamMonitor;
use crate::toolsmith::ToolsmithPlanner;
use crate::transformer::TransformerPlanner;

use super::embedder::TextEmbedder;

#[derive(Debug, Clone)]
pub struct NoironEngine {
    pub router: NoironRouter,
    pub cache: KvFusionCache,
    pub hierarchy: HierarchyController,
    pub tiered_cache: TieredCacheScheduler,
    pub infini_memory_planner: InfiniMemoryPlanner,
    pub hardware_allocator: HardwareAllocator,
    pub hardware_snapshot: HardwareSnapshot,
    pub homeostatic_setpoints: HomeostaticSetpoints,
    pub recursive_scheduler: RecursiveScheduler,
    pub stream_monitor: TokenStreamMonitor,
    pub transformer_planner: TransformerPlanner,
    pub toolsmith_planner: ToolsmithPlanner,
    pub agent_team_planner: AgentTeamPlanner,
    pub experience: ExperienceStore,
    pub experience_replay_planner: ExperienceReplayPlanner,
    pub gist_generator: GistGenerator,
    pub process_rewarder: ProcessRewarder,
    pub drift_guard: DriftGuard,
    pub reflector: Reflector,
    pub auto_replay_limit: usize,
    pub memory_retention_policy: MemoryRetentionPolicy,
    pub memory_compaction_policy: MemoryCompactionPolicy,
    pub evolution_ledger: EvolutionLedger,
    pub genome_runtime_state: GenomeRuntimeState,
    pub(super) last_tier_plan: TieredCachePlan,
    pub(super) embedder: TextEmbedder,
}

impl Default for NoironEngine {
    fn default() -> Self {
        Self {
            router: NoironRouter::new(),
            cache: KvFusionCache::new(),
            hierarchy: HierarchyController::new(),
            tiered_cache: TieredCacheScheduler::new(),
            infini_memory_planner: InfiniMemoryPlanner::new(),
            hardware_allocator: HardwareAllocator::new(),
            hardware_snapshot: HardwareSnapshot::default(),
            homeostatic_setpoints: HomeostaticSetpoints::default(),
            recursive_scheduler: RecursiveScheduler::default(),
            stream_monitor: TokenStreamMonitor::default(),
            transformer_planner: TransformerPlanner::default(),
            toolsmith_planner: ToolsmithPlanner::new(),
            agent_team_planner: AgentTeamPlanner::new(),
            experience: ExperienceStore::new(),
            experience_replay_planner: ExperienceReplayPlanner::new(),
            gist_generator: GistGenerator::new(),
            process_rewarder: ProcessRewarder::new(),
            drift_guard: DriftGuard::new(),
            reflector: Reflector::new(),
            auto_replay_limit: 2,
            memory_retention_policy: MemoryRetentionPolicy::default(),
            memory_compaction_policy: MemoryCompactionPolicy::default(),
            evolution_ledger: EvolutionLedger::default(),
            genome_runtime_state: GenomeRuntimeState::default(),
            last_tier_plan: TieredCachePlan::default(),
            embedder: TextEmbedder::default(),
        }
    }
}

impl NoironEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_cache(cache: KvFusionCache) -> Self {
        Self {
            cache,
            ..Self::default()
        }
    }
}
