use std::collections::HashMap;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryTier {
    HotGpu,
    WarmRam,
    ColdDisk,
}

impl MemoryTier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::HotGpu => "hot_gpu",
            Self::WarmRam => "warm_ram",
            Self::ColdDisk => "cold_disk",
        }
    }

    pub fn rank(self) -> u8 {
        match self {
            Self::HotGpu => 0,
            Self::WarmRam => 1,
            Self::ColdDisk => 2,
        }
    }
}

impl FromStr for MemoryTier {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "hot_gpu" | "hot-gpu" | "gpu" => Ok(Self::HotGpu),
            "warm_ram" | "warm-ram" | "ram" => Ok(Self::WarmRam),
            "cold_disk" | "cold-disk" | "disk" => Ok(Self::ColdDisk),
            other => Err(format!("unknown memory tier: {other}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TieredMemoryCandidate {
    pub id: u64,
    pub strength: f32,
    pub hits: u64,
    pub failures: u64,
    pub last_score: f32,
    pub active_similarity: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TieredMemoryCandidateSummary {
    pub id: u64,
    pub strength: f32,
    pub reliability: f32,
    pub attempts: u64,
    pub failures: u64,
    pub last_score: f32,
    pub active_similarity: f32,
}

impl TieredMemoryCandidate {
    pub fn new(id: u64, strength: f32) -> Self {
        Self {
            id,
            strength: strength.max(0.0),
            hits: 0,
            failures: 0,
            last_score: 0.0,
            active_similarity: 0.0,
        }
    }

    pub fn with_feedback(mut self, hits: u64, failures: u64, last_score: f32) -> Self {
        self.hits = hits;
        self.failures = failures;
        self.last_score = last_score;
        self
    }

    pub fn with_active_similarity(mut self, active_similarity: f32) -> Self {
        self.active_similarity = active_similarity.clamp(0.0, 1.0);
        self
    }

    pub fn reliability(&self) -> f32 {
        let attempts = self.hits.saturating_add(self.failures);
        if attempts == 0 {
            0.5
        } else {
            self.hits as f32 / attempts as f32
        }
    }

    pub fn candidate_summary(&self) -> TieredMemoryCandidateSummary {
        TieredMemoryCandidateSummary {
            id: self.id,
            strength: self.strength,
            reliability: self.reliability(),
            attempts: self.hits.saturating_add(self.failures),
            failures: self.failures,
            last_score: self.last_score,
            active_similarity: self.active_similarity,
        }
    }
}

impl TieredMemoryCandidateSummary {
    pub fn has_feedback(self) -> bool {
        self.attempts > 0
    }

    pub fn has_failures(self) -> bool {
        self.failures > 0
    }

    pub fn has_active_similarity(self) -> bool {
        self.active_similarity > 0.0
    }

    pub fn is_active_match(self, threshold: f32) -> bool {
        self.active_similarity >= threshold.clamp(0.0, 1.0)
    }

    pub fn is_failure_heavy(self) -> bool {
        self.has_feedback() && self.failures.saturating_mul(2) >= self.attempts
    }

    pub fn strength_shape_is_valid(self) -> bool {
        self.strength.is_finite() && self.strength >= 0.0
    }

    pub fn reliability_shape_is_valid(self) -> bool {
        self.reliability.is_finite() && (0.0..=1.0).contains(&self.reliability)
    }

    pub fn failure_count_shape_is_valid(self) -> bool {
        self.failures <= self.attempts
    }

    pub fn score_shape_is_valid(self) -> bool {
        self.last_score.is_finite()
    }

    pub fn active_similarity_shape_is_valid(self) -> bool {
        self.active_similarity.is_finite() && (0.0..=1.0).contains(&self.active_similarity)
    }

    pub fn candidate_signal_component_count(self) -> usize {
        usize::from(self.strength > 0.0 && self.strength_shape_is_valid())
            .saturating_add(usize::from(self.has_feedback()))
            .saturating_add(usize::from(self.has_failures()))
            .saturating_add(usize::from(self.has_active_similarity()))
            .saturating_add(usize::from(self.is_failure_heavy()))
    }

    pub fn has_candidate_signals(self) -> bool {
        self.candidate_signal_component_count() > 0
    }

    pub fn candidate_problem_component_count(self) -> usize {
        usize::from(!self.strength_shape_is_valid())
            .saturating_add(usize::from(!self.reliability_shape_is_valid()))
            .saturating_add(usize::from(!self.failure_count_shape_is_valid()))
            .saturating_add(usize::from(!self.score_shape_is_valid()))
            .saturating_add(usize::from(!self.active_similarity_shape_is_valid()))
    }

    pub fn has_candidate_problem_components(self) -> bool {
        self.candidate_problem_component_count() > 0
    }

    pub fn candidate_accounting_is_consistent(self) -> bool {
        let expected_signal_count =
            usize::from(self.strength > 0.0 && self.strength_shape_is_valid())
                .saturating_add(usize::from(self.has_feedback()))
                .saturating_add(usize::from(self.has_failures()))
                .saturating_add(usize::from(self.has_active_similarity()))
                .saturating_add(usize::from(self.is_failure_heavy()));
        let expected_problem_count = usize::from(!self.strength_shape_is_valid())
            .saturating_add(usize::from(!self.reliability_shape_is_valid()))
            .saturating_add(usize::from(!self.failure_count_shape_is_valid()))
            .saturating_add(usize::from(!self.score_shape_is_valid()))
            .saturating_add(usize::from(!self.active_similarity_shape_is_valid()));

        self.candidate_signal_component_count() == expected_signal_count
            && self.candidate_problem_component_count() == expected_problem_count
            && self.has_candidate_problem_components() == (expected_problem_count > 0)
    }

    pub fn candidate_shape_is_clean(self) -> bool {
        !self.has_candidate_problem_components() && self.candidate_accounting_is_consistent()
    }

    pub fn can_use_tiered_memory_candidate(self) -> bool {
        self.candidate_shape_is_clean() && self.strength > 0.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryPlacement {
    pub id: u64,
    pub tier: MemoryTier,
    pub score: f32,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TierMigrationAction {
    New,
    Promote,
    Demote,
    Retain,
    Evict,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TierMigration {
    pub id: u64,
    pub from: Option<MemoryTier>,
    pub to: Option<MemoryTier>,
    pub action: TierMigrationAction,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TierMigrationSummary {
    pub migration_count: usize,
    pub new: usize,
    pub promoted: usize,
    pub demoted: usize,
    pub retained: usize,
    pub evicted: usize,
}

impl TierMigrationSummary {
    pub fn from_migrations(migrations: &[TierMigration]) -> Self {
        let mut summary = Self::default();
        summary.migration_count = migrations.len();
        for migration in migrations {
            match migration.action {
                TierMigrationAction::New => summary.new += 1,
                TierMigrationAction::Promote => summary.promoted += 1,
                TierMigrationAction::Demote => summary.demoted += 1,
                TierMigrationAction::Retain => summary.retained += 1,
                TierMigrationAction::Evict => summary.evicted += 1,
            }
        }
        summary
    }

    pub fn total(self) -> usize {
        self.new + self.promoted + self.demoted + self.retained + self.evicted
    }

    pub fn migration_count_matches_actions(self) -> bool {
        self.total() == self.migration_count
    }

    pub fn changed(self) -> usize {
        self.new + self.promoted + self.demoted + self.evicted
    }

    pub fn is_noop(self) -> bool {
        self.changed() == 0
    }

    pub fn changes_match_total(self) -> bool {
        self.changed().saturating_add(self.retained) == self.migration_count
            && self.migration_count_matches_actions()
    }

    pub fn is_clean_noop(self) -> bool {
        self.is_noop() && self.migration_accounting_is_consistent()
    }

    pub fn has_new(self) -> bool {
        self.new > 0
    }

    pub fn has_promotions(self) -> bool {
        self.promoted > 0
    }

    pub fn has_demotions(self) -> bool {
        self.demoted > 0
    }

    pub fn has_evictions(self) -> bool {
        self.evicted > 0
    }

    pub fn has_tier_movement(self) -> bool {
        self.has_promotions() || self.has_demotions()
    }

    pub fn has_capacity_pressure(self) -> bool {
        self.has_demotions() || self.has_evictions()
    }

    pub fn new_entry_signal_component_count(self) -> usize {
        usize::from(self.has_new())
    }

    pub fn tier_movement_signal_component_count(self) -> usize {
        usize::from(self.has_tier_movement())
    }

    pub fn capacity_pressure_signal_component_count(self) -> usize {
        usize::from(self.has_capacity_pressure())
    }

    pub fn migration_signal_component_count(self) -> usize {
        self.new_entry_signal_component_count()
            .saturating_add(self.tier_movement_signal_component_count())
            .saturating_add(self.capacity_pressure_signal_component_count())
    }

    pub fn has_migration_signals(self) -> bool {
        self.migration_signal_component_count() > 0
    }

    pub fn migration_action_count_drift_component_count(self) -> usize {
        usize::from(!self.migration_count_matches_actions())
    }

    pub fn migration_retention_balance_drift_component_count(self) -> usize {
        usize::from(!self.changes_match_total())
    }

    pub fn migration_accounting_drift_component_count(self) -> usize {
        self.migration_action_count_drift_component_count()
            .saturating_add(self.migration_retention_balance_drift_component_count())
    }

    pub fn has_migration_accounting_drift_components(self) -> bool {
        self.migration_accounting_drift_component_count() > 0
    }

    pub fn migration_accounting_is_consistent(self) -> bool {
        let expected_drift_count = usize::from(!self.migration_count_matches_actions())
            .saturating_add(usize::from(!self.changes_match_total()));

        self.migration_accounting_drift_component_count() == expected_drift_count
            && self.has_migration_accounting_drift_components() == (expected_drift_count > 0)
    }

    pub fn migration_boundary_problem_component_count(self) -> usize {
        self.migration_accounting_drift_component_count()
    }

    pub fn has_migration_boundary_problem_components(self) -> bool {
        self.migration_boundary_problem_component_count() > 0
    }

    pub fn tier_migration_commit_signal_component_count(self) -> usize {
        self.migration_signal_component_count()
    }

    pub fn has_tier_migration_commit_signals(self) -> bool {
        self.tier_migration_commit_signal_component_count() > 0
    }

    pub fn tier_migration_commit_blocker_component_count(self) -> usize {
        self.migration_boundary_problem_component_count()
    }

    pub fn has_tier_migration_commit_blockers(self) -> bool {
        self.tier_migration_commit_blocker_component_count() > 0
    }

    pub fn tier_migration_commit_accounting_is_consistent(self) -> bool {
        self.migration_accounting_is_consistent()
            && self.tier_migration_commit_signal_component_count()
                == self.migration_signal_component_count()
            && self.has_tier_migration_commit_signals()
                == (self.tier_migration_commit_signal_component_count() > 0)
            && self.tier_migration_commit_blocker_component_count()
                == self.migration_boundary_problem_component_count()
            && self.has_tier_migration_commit_blockers()
                == (self.tier_migration_commit_blocker_component_count() > 0)
    }

    pub fn tier_migration_commit_is_clean(self) -> bool {
        !self.has_tier_migration_commit_blockers()
            && self.tier_migration_commit_accounting_is_consistent()
    }

    pub fn migration_commit_is_clean(self) -> bool {
        self.tier_migration_commit_is_clean()
    }

    pub fn can_commit_tier_migration(self) -> bool {
        self.tier_migration_commit_is_clean()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TierCounts {
    pub hot_gpu: usize,
    pub warm_ram: usize,
    pub cold_disk: usize,
}

impl TierCounts {
    pub fn total(self) -> usize {
        self.hot_gpu
            .saturating_add(self.warm_ram)
            .saturating_add(self.cold_disk)
    }

    pub fn has_hot(self) -> bool {
        self.hot_gpu > 0
    }

    pub fn has_warm(self) -> bool {
        self.warm_ram > 0
    }

    pub fn has_cold(self) -> bool {
        self.cold_disk > 0
    }

    pub fn active_tier_count(self) -> usize {
        [self.has_hot(), self.has_warm(), self.has_cold()]
            .into_iter()
            .filter(|present| *present)
            .count()
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TieredCachePlan {
    placements: Vec<MemoryPlacement>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TieredCacheSummary {
    pub placement_count: usize,
    pub counts: TierCounts,
    pub average_score: f32,
    pub min_score: f32,
    pub max_score: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TieredCachePlacementCommitAction {
    CommitTieredCachePlacement,
    WaitForTieredCachePlacement,
    RepairTieredCachePlacement,
}

impl TieredCachePlacementCommitAction {
    pub fn can_commit(self) -> bool {
        matches!(self, Self::CommitTieredCachePlacement)
    }

    pub fn should_wait(self) -> bool {
        matches!(self, Self::WaitForTieredCachePlacement)
    }

    pub fn should_repair(self) -> bool {
        matches!(self, Self::RepairTieredCachePlacement)
    }
}

impl TieredCacheSummary {
    pub fn is_empty(self) -> bool {
        self.placement_count == 0
    }

    pub fn fraction_for(self, tier: MemoryTier) -> f32 {
        if self.placement_count == 0 {
            return 0.0;
        }

        let count = match tier {
            MemoryTier::HotGpu => self.counts.hot_gpu,
            MemoryTier::WarmRam => self.counts.warm_ram,
            MemoryTier::ColdDisk => self.counts.cold_disk,
        };
        count as f32 / self.placement_count as f32
    }

    pub fn hot_fraction(self) -> f32 {
        self.fraction_for(MemoryTier::HotGpu)
    }

    pub fn warm_fraction(self) -> f32 {
        self.fraction_for(MemoryTier::WarmRam)
    }

    pub fn cold_fraction(self) -> f32 {
        self.fraction_for(MemoryTier::ColdDisk)
    }

    pub fn has_hot(self) -> bool {
        self.counts.has_hot()
    }

    pub fn has_warm(self) -> bool {
        self.counts.has_warm()
    }

    pub fn has_cold(self) -> bool {
        self.counts.has_cold()
    }

    pub fn uses_multiple_tiers(self) -> bool {
        self.counts.active_tier_count() > 1
    }

    pub fn counts_match_placements(self) -> bool {
        self.counts.total() == self.placement_count
    }

    pub fn has_score_spread(self) -> bool {
        !self.is_empty() && (self.max_score - self.min_score).abs() > f32::EPSILON
    }

    pub fn all_hot(self) -> bool {
        !self.is_empty() && self.counts.hot_gpu == self.placement_count
    }

    pub fn all_warm(self) -> bool {
        !self.is_empty() && self.counts.warm_ram == self.placement_count
    }

    pub fn all_cold(self) -> bool {
        !self.is_empty() && self.counts.cold_disk == self.placement_count
    }

    pub fn cold_dominates_hot(self) -> bool {
        self.counts.cold_disk > self.counts.hot_gpu
    }

    pub fn multi_tier_signal_component_count(self) -> usize {
        usize::from(self.uses_multiple_tiers())
    }

    pub fn score_spread_signal_component_count(self) -> usize {
        usize::from(self.has_score_spread())
    }

    pub fn cold_pressure_signal_component_count(self) -> usize {
        usize::from(self.cold_dominates_hot())
    }

    pub fn cache_distribution_signal_component_count(self) -> usize {
        self.multi_tier_signal_component_count()
            .saturating_add(self.score_spread_signal_component_count())
            .saturating_add(self.cold_pressure_signal_component_count())
    }

    pub fn has_cache_distribution_signals(self) -> bool {
        self.cache_distribution_signal_component_count() > 0
    }

    pub fn cache_placement_signal_component_count(self) -> usize {
        self.cache_distribution_signal_component_count()
    }

    pub fn has_cache_placement_signals(self) -> bool {
        self.cache_placement_signal_component_count() > 0
    }

    pub fn score_fields_are_finite(self) -> bool {
        self.average_score.is_finite() && self.min_score.is_finite() && self.max_score.is_finite()
    }

    pub fn score_bounds_are_ordered(self) -> bool {
        self.is_empty() || self.min_score <= self.max_score
    }

    pub fn empty_score_fields_are_zero(self) -> bool {
        !self.is_empty()
            || (self.average_score.abs() <= f32::EPSILON
                && self.min_score.abs() <= f32::EPSILON
                && self.max_score.abs() <= f32::EPSILON)
    }

    pub fn average_score_within_bounds(self) -> bool {
        if self.is_empty() {
            return true;
        }

        self.score_fields_are_finite()
            && self.average_score + f32::EPSILON >= self.min_score
            && self.average_score - f32::EPSILON <= self.max_score
    }

    pub fn placement_count_drift_component_count(self) -> usize {
        usize::from(!self.counts_match_placements())
    }

    pub fn non_finite_score_component_count(self) -> usize {
        usize::from(!self.score_fields_are_finite())
    }

    pub fn score_bounds_drift_component_count(self) -> usize {
        usize::from(!self.score_bounds_are_ordered())
    }

    pub fn empty_score_shape_drift_component_count(self) -> usize {
        usize::from(!self.empty_score_fields_are_zero())
    }

    pub fn average_score_drift_component_count(self) -> usize {
        usize::from(!self.average_score_within_bounds())
    }

    pub fn score_shape_problem_component_count(self) -> usize {
        self.non_finite_score_component_count()
            .saturating_add(self.score_bounds_drift_component_count())
            .saturating_add(self.empty_score_shape_drift_component_count())
            .saturating_add(self.average_score_drift_component_count())
    }

    pub fn cache_summary_problem_component_count(self) -> usize {
        self.placement_count_drift_component_count()
            .saturating_add(self.score_shape_problem_component_count())
    }

    pub fn has_cache_summary_problem_components(self) -> bool {
        self.cache_summary_problem_component_count() > 0
    }

    pub fn cache_placement_blocker_component_count(self) -> usize {
        self.cache_summary_problem_component_count()
    }

    pub fn has_cache_placement_blockers(self) -> bool {
        self.cache_placement_blocker_component_count() > 0
    }

    pub fn cache_summary_accounting_is_consistent(self) -> bool {
        let expected_problem_count = usize::from(!self.counts_match_placements())
            .saturating_add(usize::from(!self.score_fields_are_finite()))
            .saturating_add(usize::from(!self.score_bounds_are_ordered()))
            .saturating_add(usize::from(!self.empty_score_fields_are_zero()))
            .saturating_add(usize::from(!self.average_score_within_bounds()));

        self.cache_summary_problem_component_count() == expected_problem_count
            && self.has_cache_summary_problem_components() == (expected_problem_count > 0)
    }

    pub fn cache_summary_is_clean(self) -> bool {
        !self.has_cache_summary_problem_components()
            && self.cache_summary_accounting_is_consistent()
    }

    pub fn cache_placement_accounting_is_consistent(self) -> bool {
        self.cache_summary_accounting_is_consistent()
            && self.cache_placement_signal_component_count()
                == self.cache_distribution_signal_component_count()
            && self.has_cache_placement_signals()
                == (self.cache_placement_signal_component_count() > 0)
            && self.cache_placement_blocker_component_count()
                == self.cache_summary_problem_component_count()
            && self.has_cache_placement_blockers()
                == (self.cache_placement_blocker_component_count() > 0)
    }

    pub fn tiered_cache_placement_commit_signal_component_count(self) -> usize {
        self.cache_placement_signal_component_count()
    }

    pub fn has_tiered_cache_placement_commit_signals(self) -> bool {
        self.tiered_cache_placement_commit_signal_component_count() > 0
    }

    pub fn tiered_cache_placement_commit_blocker_component_count(self) -> usize {
        self.cache_placement_blocker_component_count()
    }

    pub fn has_tiered_cache_placement_commit_blockers(self) -> bool {
        self.tiered_cache_placement_commit_blocker_component_count() > 0
    }

    pub fn tiered_cache_placement_commit_accounting_is_consistent(self) -> bool {
        self.cache_placement_accounting_is_consistent()
            && self.tiered_cache_placement_commit_signal_component_count()
                == self.cache_placement_signal_component_count()
            && self.has_tiered_cache_placement_commit_signals()
                == (self.tiered_cache_placement_commit_signal_component_count() > 0)
            && self.tiered_cache_placement_commit_blocker_component_count()
                == self.cache_placement_blocker_component_count()
            && self.has_tiered_cache_placement_commit_blockers()
                == (self.tiered_cache_placement_commit_blocker_component_count() > 0)
    }

    pub fn tiered_cache_placement_commit_is_clean(self) -> bool {
        !self.has_tiered_cache_placement_commit_blockers()
            && self.tiered_cache_placement_commit_accounting_is_consistent()
    }

    pub fn cache_placement_commit_is_clean(self) -> bool {
        self.tiered_cache_placement_commit_is_clean()
    }

    pub fn can_commit_tiered_cache_placement(self) -> bool {
        self.tiered_cache_placement_commit_is_clean() && !self.is_empty()
    }

    pub fn tiered_cache_placement_commit_action(self) -> TieredCachePlacementCommitAction {
        if self.can_commit_tiered_cache_placement() {
            TieredCachePlacementCommitAction::CommitTieredCachePlacement
        } else if self.has_tiered_cache_placement_commit_blockers() {
            TieredCachePlacementCommitAction::RepairTieredCachePlacement
        } else {
            TieredCachePlacementCommitAction::WaitForTieredCachePlacement
        }
    }

    pub fn can_use_tiered_cache_summary(self) -> bool {
        self.cache_summary_is_clean() && !self.is_empty()
    }
}

impl TieredCachePlan {
    pub fn new(placements: Vec<MemoryPlacement>) -> Self {
        Self { placements }
    }

    pub fn placements(&self) -> &[MemoryPlacement] {
        &self.placements
    }

    pub fn placement_for(&self, id: u64) -> Option<&MemoryPlacement> {
        self.placements.iter().find(|placement| placement.id == id)
    }

    pub fn migrations_from(&self, previous: &TieredCachePlan) -> Vec<TierMigration> {
        let previous_by_id = previous
            .placements
            .iter()
            .map(|placement| (placement.id, placement))
            .collect::<HashMap<_, _>>();
        let current_by_id = self
            .placements
            .iter()
            .map(|placement| (placement.id, placement))
            .collect::<HashMap<_, _>>();
        let mut migrations = Vec::new();

        for current in &self.placements {
            let Some(previous) = previous_by_id.get(&current.id) else {
                migrations.push(TierMigration {
                    id: current.id,
                    from: None,
                    to: Some(current.tier),
                    action: TierMigrationAction::New,
                    reason: format!("new:{}", current.reason),
                });
                continue;
            };

            let action = if current.tier.rank() < previous.tier.rank() {
                TierMigrationAction::Promote
            } else if current.tier.rank() > previous.tier.rank() {
                TierMigrationAction::Demote
            } else {
                TierMigrationAction::Retain
            };

            migrations.push(TierMigration {
                id: current.id,
                from: Some(previous.tier),
                to: Some(current.tier),
                action,
                reason: format!("{} -> {}", previous.reason, current.reason),
            });
        }

        for previous in &previous.placements {
            if !current_by_id.contains_key(&previous.id) {
                migrations.push(TierMigration {
                    id: previous.id,
                    from: Some(previous.tier),
                    to: None,
                    action: TierMigrationAction::Evict,
                    reason: format!("evict:{}", previous.reason),
                });
            }
        }

        migrations
    }

    pub fn migration_summary_from(&self, previous: &TieredCachePlan) -> TierMigrationSummary {
        TierMigrationSummary::from_migrations(&self.migrations_from(previous))
    }

    pub fn counts(&self) -> TierCounts {
        let mut counts = TierCounts::default();

        for placement in &self.placements {
            match placement.tier {
                MemoryTier::HotGpu => counts.hot_gpu += 1,
                MemoryTier::WarmRam => counts.warm_ram += 1,
                MemoryTier::ColdDisk => counts.cold_disk += 1,
            }
        }

        counts
    }

    pub fn summary(&self) -> TieredCacheSummary {
        let counts = self.counts();
        let placement_count = self.placements.len();
        let total_score = self
            .placements
            .iter()
            .map(|placement| placement.score)
            .sum::<f32>();
        let average_score = if placement_count == 0 {
            0.0
        } else {
            total_score / placement_count as f32
        };
        let min_score = self
            .placements
            .iter()
            .map(|placement| placement.score)
            .min_by(|left, right| left.total_cmp(right))
            .unwrap_or(0.0);
        let max_score = self
            .placements
            .iter()
            .map(|placement| placement.score)
            .max_by(|left, right| left.total_cmp(right))
            .unwrap_or(0.0);

        TieredCacheSummary {
            placement_count,
            counts,
            average_score,
            min_score,
            max_score,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TieredMemoryScheduler {
    pub hot_capacity: usize,
    pub warm_capacity: usize,
    pub hot_threshold: f32,
    pub warm_threshold: f32,
    pub active_boost: f32,
    pub failure_penalty: f32,
}

impl TieredMemoryScheduler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacities(hot_capacity: usize, warm_capacity: usize) -> Self {
        Self {
            hot_capacity,
            warm_capacity,
            ..Self::default()
        }
    }

    pub fn plan(&self, candidates: &[TieredMemoryCandidate]) -> TieredCachePlan {
        let mut scored = candidates
            .iter()
            .map(|candidate| (candidate.id, self.score_candidate(candidate), candidate))
            .collect::<Vec<_>>();

        scored.sort_by(|(left_id, left_score, _), (right_id, right_score, _)| {
            right_score
                .total_cmp(left_score)
                .then_with(|| left_id.cmp(right_id))
        });

        let placements = scored
            .into_iter()
            .enumerate()
            .map(|(rank, (id, score, candidate))| {
                let tier = self.assign_tier(rank, score);
                MemoryPlacement {
                    id,
                    tier,
                    score,
                    reason: placement_reason(tier, score, candidate.active_similarity),
                }
            })
            .collect();

        TieredCachePlan::new(placements)
    }

    pub fn score_candidate(&self, candidate: &TieredMemoryCandidate) -> f32 {
        let failure_drag = candidate.failures as f32 * self.failure_penalty;

        (candidate.strength * 0.45
            + candidate.last_score.max(0.0) * 0.18
            + candidate.reliability() * 0.22
            + candidate.active_similarity * self.active_boost
            - failure_drag)
            .clamp(0.0, 3.0)
    }

    pub fn assign_tier(&self, rank: usize, score: f32) -> MemoryTier {
        if rank < self.hot_capacity && score >= self.hot_threshold {
            MemoryTier::HotGpu
        } else if rank < self.hot_capacity + self.warm_capacity && score >= self.warm_threshold {
            MemoryTier::WarmRam
        } else {
            MemoryTier::ColdDisk
        }
    }
}

impl Default for TieredMemoryScheduler {
    fn default() -> Self {
        Self {
            hot_capacity: 8,
            warm_capacity: 64,
            hot_threshold: 0.85,
            warm_threshold: 0.32,
            active_boost: 0.55,
            failure_penalty: 0.08,
        }
    }
}

fn placement_reason(tier: MemoryTier, score: f32, active_similarity: f32) -> String {
    format!(
        "{}:score={score:.3}:active_similarity={active_similarity:.3}",
        tier.as_str()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_strong_memory_is_promoted_hot() {
        let scheduler = TieredMemoryScheduler::with_capacities(1, 2);
        let candidates = vec![
            TieredMemoryCandidate::new(1, 1.4)
                .with_feedback(4, 0, 0.95)
                .with_active_similarity(0.9),
            TieredMemoryCandidate::new(2, 0.7).with_feedback(1, 0, 0.65),
        ];

        let plan = scheduler.plan(&candidates);
        let summary = plan.summary();
        let candidate_summary = candidates[0].candidate_summary();

        assert_eq!(candidate_summary.id, 1);
        assert_eq!(candidate_summary.strength, 1.4);
        assert_eq!(candidate_summary.reliability, 1.0);
        assert_eq!(candidate_summary.attempts, 4);
        assert_eq!(candidate_summary.failures, 0);
        assert_eq!(candidate_summary.last_score, 0.95);
        assert_eq!(candidate_summary.active_similarity, 0.9);
        assert!(candidate_summary.has_feedback());
        assert!(!candidate_summary.has_failures());
        assert!(candidate_summary.has_active_similarity());
        assert!(candidate_summary.is_active_match(0.80));
        assert!(!candidate_summary.is_failure_heavy());
        assert!(candidate_summary.candidate_shape_is_clean());
        assert!(candidate_summary.can_use_tiered_memory_candidate());
        assert_eq!(plan.placement_for(1).unwrap().tier, MemoryTier::HotGpu);
        assert_eq!(plan.counts().hot_gpu, 1);
        assert_eq!(summary.placement_count, 2);
        assert_eq!(summary.counts.total(), 2);
        assert_eq!(summary.counts.hot_gpu, 1);
        assert!((summary.hot_fraction() - 0.5).abs() < f32::EPSILON);
        assert!(summary.average_score > 0.0);
        assert!(summary.max_score >= summary.min_score);
        assert!(!summary.is_empty());
    }

    #[test]
    fn weak_failed_memory_goes_cold() {
        let scheduler = TieredMemoryScheduler::with_capacities(2, 2);
        let candidates = vec![TieredMemoryCandidate::new(7, 0.08).with_feedback(0, 5, 0.1)];

        let plan = scheduler.plan(&candidates);
        let candidate_summary = candidates[0].candidate_summary();

        assert_eq!(candidate_summary.id, 7);
        assert_eq!(candidate_summary.reliability, 0.0);
        assert_eq!(candidate_summary.attempts, 5);
        assert_eq!(candidate_summary.failures, 5);
        assert!(candidate_summary.has_feedback());
        assert!(candidate_summary.has_failures());
        assert!(!candidate_summary.has_active_similarity());
        assert!(!candidate_summary.is_active_match(0.1));
        assert!(candidate_summary.is_failure_heavy());
        assert!(candidate_summary.strength_shape_is_valid());
        assert!(candidate_summary.reliability_shape_is_valid());
        assert!(candidate_summary.failure_count_shape_is_valid());
        assert!(candidate_summary.score_shape_is_valid());
        assert!(candidate_summary.active_similarity_shape_is_valid());
        assert_eq!(candidate_summary.candidate_signal_component_count(), 4);
        assert!(candidate_summary.has_candidate_signals());
        assert_eq!(candidate_summary.candidate_problem_component_count(), 0);
        assert!(!candidate_summary.has_candidate_problem_components());
        assert!(candidate_summary.candidate_accounting_is_consistent());
        assert!(candidate_summary.candidate_shape_is_clean());
        assert!(candidate_summary.can_use_tiered_memory_candidate());
        assert_eq!(plan.placement_for(7).unwrap().tier, MemoryTier::ColdDisk);
    }

    #[test]
    fn tiered_memory_candidate_summary_counts_public_shape_drift() {
        let summary = TieredMemoryCandidateSummary {
            id: 1,
            strength: f32::NAN,
            reliability: 1.2,
            attempts: 2,
            failures: 3,
            last_score: f32::INFINITY,
            active_similarity: -0.1,
        };

        assert!(!summary.strength_shape_is_valid());
        assert!(!summary.reliability_shape_is_valid());
        assert!(!summary.failure_count_shape_is_valid());
        assert!(!summary.score_shape_is_valid());
        assert!(!summary.active_similarity_shape_is_valid());
        assert_eq!(summary.candidate_signal_component_count(), 3);
        assert!(summary.has_candidate_signals());
        assert_eq!(summary.candidate_problem_component_count(), 5);
        assert!(summary.has_candidate_problem_components());
        assert!(summary.candidate_accounting_is_consistent());
        assert!(!summary.candidate_shape_is_clean());
        assert!(!summary.can_use_tiered_memory_candidate());
    }

    #[test]
    fn tiered_cache_summary_reports_complete_distribution() {
        let plan = TieredCachePlan::new(vec![
            placement(1, MemoryTier::HotGpu),
            placement(2, MemoryTier::WarmRam),
            placement(3, MemoryTier::WarmRam),
            placement(4, MemoryTier::ColdDisk),
        ]);

        let summary = plan.summary();

        assert_eq!(summary.placement_count, 4);
        assert_eq!(summary.counts.hot_gpu, 1);
        assert_eq!(summary.counts.warm_ram, 2);
        assert_eq!(summary.counts.cold_disk, 1);
        assert_eq!(summary.counts.active_tier_count(), 3);
        assert!((summary.fraction_for(MemoryTier::HotGpu) - 0.25).abs() < f32::EPSILON);
        assert!((summary.hot_fraction() - 0.25).abs() < f32::EPSILON);
        assert!((summary.warm_fraction() - 0.50).abs() < f32::EPSILON);
        assert!((summary.cold_fraction() - 0.25).abs() < f32::EPSILON);
        assert!(summary.has_hot());
        assert!(summary.has_warm());
        assert!(summary.has_cold());
        assert!(summary.uses_multiple_tiers());
        assert!(summary.counts_match_placements());
        assert!(summary.has_score_spread());
        assert!(!summary.all_hot());
        assert!(!summary.all_warm());
        assert!(!summary.all_cold());
        assert!(!summary.cold_dominates_hot());
        assert_eq!(summary.multi_tier_signal_component_count(), 1);
        assert_eq!(summary.score_spread_signal_component_count(), 1);
        assert_eq!(summary.cold_pressure_signal_component_count(), 0);
        assert_eq!(summary.cache_distribution_signal_component_count(), 2);
        assert!(summary.has_cache_distribution_signals());
        assert_eq!(summary.cache_placement_signal_component_count(), 2);
        assert!(summary.has_cache_placement_signals());
        assert!(summary.score_fields_are_finite());
        assert!(summary.score_bounds_are_ordered());
        assert!(summary.empty_score_fields_are_zero());
        assert!(summary.average_score_within_bounds());
        assert_eq!(summary.placement_count_drift_component_count(), 0);
        assert_eq!(summary.non_finite_score_component_count(), 0);
        assert_eq!(summary.score_bounds_drift_component_count(), 0);
        assert_eq!(summary.empty_score_shape_drift_component_count(), 0);
        assert_eq!(summary.average_score_drift_component_count(), 0);
        assert_eq!(summary.score_shape_problem_component_count(), 0);
        assert_eq!(summary.cache_summary_problem_component_count(), 0);
        assert!(!summary.has_cache_summary_problem_components());
        assert_eq!(summary.cache_placement_blocker_component_count(), 0);
        assert!(!summary.has_cache_placement_blockers());
        assert!(summary.cache_summary_accounting_is_consistent());
        assert!(summary.cache_summary_is_clean());
        assert!(summary.cache_placement_accounting_is_consistent());
        assert_eq!(
            summary.tiered_cache_placement_commit_signal_component_count(),
            2
        );
        assert!(summary.has_tiered_cache_placement_commit_signals());
        assert_eq!(
            summary.tiered_cache_placement_commit_blocker_component_count(),
            0
        );
        assert!(!summary.has_tiered_cache_placement_commit_blockers());
        assert!(summary.tiered_cache_placement_commit_accounting_is_consistent());
        assert!(summary.tiered_cache_placement_commit_is_clean());
        assert!(summary.cache_placement_commit_is_clean());
        assert!(summary.can_commit_tiered_cache_placement());
        assert_eq!(
            summary.tiered_cache_placement_commit_action(),
            TieredCachePlacementCommitAction::CommitTieredCachePlacement
        );
        assert!(summary.tiered_cache_placement_commit_action().can_commit());
        assert!(!summary.tiered_cache_placement_commit_action().should_wait());
        assert!(
            !summary
                .tiered_cache_placement_commit_action()
                .should_repair()
        );
        assert!(summary.can_use_tiered_cache_summary());
    }

    #[test]
    fn empty_tiered_cache_summary_has_zero_distribution() {
        let summary = TieredCachePlan::default().summary();

        assert!(summary.is_empty());
        assert_eq!(summary.counts.total(), 0);
        assert_eq!(summary.hot_fraction(), 0.0);
        assert_eq!(summary.warm_fraction(), 0.0);
        assert_eq!(summary.cold_fraction(), 0.0);
        assert!(!summary.has_hot());
        assert!(!summary.has_warm());
        assert!(!summary.has_cold());
        assert!(!summary.uses_multiple_tiers());
        assert!(summary.counts_match_placements());
        assert!(!summary.has_score_spread());
        assert!(!summary.all_hot());
        assert!(!summary.all_warm());
        assert!(!summary.all_cold());
        assert!(!summary.cold_dominates_hot());
        assert_eq!(summary.multi_tier_signal_component_count(), 0);
        assert_eq!(summary.score_spread_signal_component_count(), 0);
        assert_eq!(summary.cold_pressure_signal_component_count(), 0);
        assert_eq!(summary.cache_distribution_signal_component_count(), 0);
        assert!(!summary.has_cache_distribution_signals());
        assert_eq!(summary.cache_placement_signal_component_count(), 0);
        assert!(!summary.has_cache_placement_signals());
        assert!(summary.score_fields_are_finite());
        assert!(summary.score_bounds_are_ordered());
        assert!(summary.empty_score_fields_are_zero());
        assert!(summary.average_score_within_bounds());
        assert_eq!(summary.placement_count_drift_component_count(), 0);
        assert_eq!(summary.non_finite_score_component_count(), 0);
        assert_eq!(summary.score_bounds_drift_component_count(), 0);
        assert_eq!(summary.empty_score_shape_drift_component_count(), 0);
        assert_eq!(summary.average_score_drift_component_count(), 0);
        assert_eq!(summary.score_shape_problem_component_count(), 0);
        assert_eq!(summary.cache_summary_problem_component_count(), 0);
        assert!(!summary.has_cache_summary_problem_components());
        assert_eq!(summary.cache_placement_blocker_component_count(), 0);
        assert!(!summary.has_cache_placement_blockers());
        assert!(summary.cache_summary_accounting_is_consistent());
        assert!(summary.cache_summary_is_clean());
        assert!(summary.cache_placement_accounting_is_consistent());
        assert_eq!(
            summary.tiered_cache_placement_commit_signal_component_count(),
            0
        );
        assert!(!summary.has_tiered_cache_placement_commit_signals());
        assert_eq!(
            summary.tiered_cache_placement_commit_blocker_component_count(),
            0
        );
        assert!(!summary.has_tiered_cache_placement_commit_blockers());
        assert!(summary.tiered_cache_placement_commit_accounting_is_consistent());
        assert!(summary.tiered_cache_placement_commit_is_clean());
        assert!(summary.cache_placement_commit_is_clean());
        assert!(!summary.can_commit_tiered_cache_placement());
        assert_eq!(
            summary.tiered_cache_placement_commit_action(),
            TieredCachePlacementCommitAction::WaitForTieredCachePlacement
        );
        assert!(!summary.tiered_cache_placement_commit_action().can_commit());
        assert!(summary.tiered_cache_placement_commit_action().should_wait());
        assert!(
            !summary
                .tiered_cache_placement_commit_action()
                .should_repair()
        );
        assert!(!summary.can_use_tiered_cache_summary());
    }

    #[test]
    fn tiered_cache_summary_reports_shape_drift() {
        let summary = TieredCacheSummary {
            placement_count: 2,
            counts: TierCounts {
                hot_gpu: 1,
                warm_ram: 0,
                cold_disk: 0,
            },
            average_score: f32::NAN,
            min_score: 3.0,
            max_score: 2.0,
        };

        assert!(!summary.is_empty());
        assert!(!summary.counts_match_placements());
        assert!(!summary.score_fields_are_finite());
        assert!(!summary.score_bounds_are_ordered());
        assert!(summary.empty_score_fields_are_zero());
        assert!(!summary.average_score_within_bounds());
        assert_eq!(summary.placement_count_drift_component_count(), 1);
        assert_eq!(summary.non_finite_score_component_count(), 1);
        assert_eq!(summary.score_bounds_drift_component_count(), 1);
        assert_eq!(summary.empty_score_shape_drift_component_count(), 0);
        assert_eq!(summary.average_score_drift_component_count(), 1);
        assert_eq!(summary.score_shape_problem_component_count(), 3);
        assert_eq!(summary.cache_summary_problem_component_count(), 4);
        assert!(summary.has_cache_summary_problem_components());
        assert_eq!(summary.cache_placement_blocker_component_count(), 4);
        assert!(summary.has_cache_placement_blockers());
        assert!(summary.cache_summary_accounting_is_consistent());
        assert!(!summary.cache_summary_is_clean());
        assert!(summary.cache_placement_accounting_is_consistent());
        assert_eq!(
            summary.tiered_cache_placement_commit_signal_component_count(),
            1
        );
        assert!(summary.has_tiered_cache_placement_commit_signals());
        assert_eq!(
            summary.tiered_cache_placement_commit_blocker_component_count(),
            4
        );
        assert!(summary.has_tiered_cache_placement_commit_blockers());
        assert!(summary.tiered_cache_placement_commit_accounting_is_consistent());
        assert!(!summary.tiered_cache_placement_commit_is_clean());
        assert!(!summary.cache_placement_commit_is_clean());
        assert!(!summary.can_commit_tiered_cache_placement());
        assert_eq!(
            summary.tiered_cache_placement_commit_action(),
            TieredCachePlacementCommitAction::RepairTieredCachePlacement
        );
        assert!(!summary.tiered_cache_placement_commit_action().can_commit());
        assert!(!summary.tiered_cache_placement_commit_action().should_wait());
        assert!(
            summary
                .tiered_cache_placement_commit_action()
                .should_repair()
        );
        assert!(!summary.can_use_tiered_cache_summary());
    }

    #[test]
    fn migrations_capture_new_promote_demote_retain_and_evict() {
        let previous = TieredCachePlan::new(vec![
            placement(1, MemoryTier::ColdDisk),
            placement(2, MemoryTier::HotGpu),
            placement(3, MemoryTier::WarmRam),
            placement(4, MemoryTier::WarmRam),
        ]);
        let current = TieredCachePlan::new(vec![
            placement(1, MemoryTier::HotGpu),
            placement(2, MemoryTier::WarmRam),
            placement(3, MemoryTier::WarmRam),
            placement(5, MemoryTier::ColdDisk),
        ]);

        let migrations = current.migrations_from(&previous);

        assert_eq!(action_for(&migrations, 1), TierMigrationAction::Promote);
        assert_eq!(action_for(&migrations, 2), TierMigrationAction::Demote);
        assert_eq!(action_for(&migrations, 3), TierMigrationAction::Retain);
        assert_eq!(action_for(&migrations, 4), TierMigrationAction::Evict);
        assert_eq!(action_for(&migrations, 5), TierMigrationAction::New);

        let summary = current.migration_summary_from(&previous);
        let cache_summary = current.summary();
        assert_eq!(
            summary,
            TierMigrationSummary {
                migration_count: 5,
                new: 1,
                promoted: 1,
                demoted: 1,
                retained: 1,
                evicted: 1,
            }
        );
        assert_eq!(summary.total(), 5);
        assert_eq!(summary.changed(), 4);
        assert!(!summary.is_noop());
        assert!(!summary.is_clean_noop());
        assert!(summary.migration_count_matches_actions());
        assert!(summary.changes_match_total());
        assert!(summary.has_new());
        assert!(summary.has_promotions());
        assert!(summary.has_demotions());
        assert!(summary.has_evictions());
        assert!(summary.has_tier_movement());
        assert!(summary.has_capacity_pressure());
        assert_eq!(summary.new_entry_signal_component_count(), 1);
        assert_eq!(summary.tier_movement_signal_component_count(), 1);
        assert_eq!(summary.capacity_pressure_signal_component_count(), 1);
        assert_eq!(summary.migration_signal_component_count(), 3);
        assert!(summary.has_migration_signals());
        assert_eq!(summary.migration_action_count_drift_component_count(), 0);
        assert_eq!(
            summary.migration_retention_balance_drift_component_count(),
            0
        );
        assert_eq!(summary.migration_accounting_drift_component_count(), 0);
        assert!(!summary.has_migration_accounting_drift_components());
        assert!(summary.migration_accounting_is_consistent());
        assert_eq!(summary.migration_boundary_problem_component_count(), 0);
        assert!(!summary.has_migration_boundary_problem_components());
        assert_eq!(summary.tier_migration_commit_signal_component_count(), 3);
        assert!(summary.has_tier_migration_commit_signals());
        assert_eq!(summary.tier_migration_commit_blocker_component_count(), 0);
        assert!(!summary.has_tier_migration_commit_blockers());
        assert!(summary.tier_migration_commit_accounting_is_consistent());
        assert!(summary.tier_migration_commit_is_clean());
        assert!(summary.migration_commit_is_clean());
        assert!(summary.can_commit_tier_migration());
        assert_eq!(cache_summary.placement_count, 4);
        assert_eq!(cache_summary.counts.total(), 4);
    }

    #[test]
    fn migration_summary_marks_retained_only_plan_as_noop() {
        let previous = TieredCachePlan::new(vec![
            placement(1, MemoryTier::HotGpu),
            placement(2, MemoryTier::WarmRam),
        ]);
        let current = TieredCachePlan::new(vec![
            placement(1, MemoryTier::HotGpu),
            placement(2, MemoryTier::WarmRam),
        ]);

        let summary = current.migration_summary_from(&previous);

        assert_eq!(summary.retained, 2);
        assert_eq!(summary.migration_count, 2);
        assert_eq!(summary.total(), 2);
        assert_eq!(summary.changed(), 0);
        assert!(summary.is_noop());
        assert!(summary.is_clean_noop());
        assert!(summary.migration_count_matches_actions());
        assert!(summary.changes_match_total());
        assert!(!summary.has_new());
        assert!(!summary.has_promotions());
        assert!(!summary.has_demotions());
        assert!(!summary.has_evictions());
        assert!(!summary.has_tier_movement());
        assert!(!summary.has_capacity_pressure());
        assert_eq!(summary.new_entry_signal_component_count(), 0);
        assert_eq!(summary.tier_movement_signal_component_count(), 0);
        assert_eq!(summary.capacity_pressure_signal_component_count(), 0);
        assert_eq!(summary.migration_signal_component_count(), 0);
        assert!(!summary.has_migration_signals());
        assert_eq!(summary.migration_action_count_drift_component_count(), 0);
        assert_eq!(
            summary.migration_retention_balance_drift_component_count(),
            0
        );
        assert_eq!(summary.migration_accounting_drift_component_count(), 0);
        assert!(!summary.has_migration_accounting_drift_components());
        assert!(summary.migration_accounting_is_consistent());
        assert_eq!(summary.migration_boundary_problem_component_count(), 0);
        assert!(!summary.has_migration_boundary_problem_components());
        assert_eq!(summary.tier_migration_commit_signal_component_count(), 0);
        assert!(!summary.has_tier_migration_commit_signals());
        assert_eq!(summary.tier_migration_commit_blocker_component_count(), 0);
        assert!(!summary.has_tier_migration_commit_blockers());
        assert!(summary.tier_migration_commit_accounting_is_consistent());
        assert!(summary.tier_migration_commit_is_clean());
        assert!(summary.migration_commit_is_clean());
        assert!(summary.can_commit_tier_migration());
    }

    #[test]
    fn migration_summary_reports_accounting_drift() {
        let summary = TierMigrationSummary {
            migration_count: 2,
            new: 1,
            promoted: 1,
            demoted: 1,
            retained: 0,
            evicted: 0,
        };

        assert_eq!(summary.total(), 3);
        assert_eq!(summary.changed(), 3);
        assert!(!summary.is_noop());
        assert!(!summary.is_clean_noop());
        assert!(!summary.migration_count_matches_actions());
        assert!(!summary.changes_match_total());
        assert!(summary.has_new());
        assert!(summary.has_promotions());
        assert!(summary.has_demotions());
        assert!(!summary.has_evictions());
        assert!(summary.has_tier_movement());
        assert!(summary.has_capacity_pressure());
        assert_eq!(summary.new_entry_signal_component_count(), 1);
        assert_eq!(summary.tier_movement_signal_component_count(), 1);
        assert_eq!(summary.capacity_pressure_signal_component_count(), 1);
        assert_eq!(summary.migration_signal_component_count(), 3);
        assert!(summary.has_migration_signals());
        assert_eq!(summary.migration_action_count_drift_component_count(), 1);
        assert_eq!(
            summary.migration_retention_balance_drift_component_count(),
            1
        );
        assert_eq!(summary.migration_accounting_drift_component_count(), 2);
        assert!(summary.has_migration_accounting_drift_components());
        assert!(summary.migration_accounting_is_consistent());
        assert_eq!(summary.migration_boundary_problem_component_count(), 2);
        assert!(summary.has_migration_boundary_problem_components());
        assert_eq!(summary.tier_migration_commit_signal_component_count(), 3);
        assert!(summary.has_tier_migration_commit_signals());
        assert_eq!(summary.tier_migration_commit_blocker_component_count(), 2);
        assert!(summary.has_tier_migration_commit_blockers());
        assert!(summary.tier_migration_commit_accounting_is_consistent());
        assert!(!summary.tier_migration_commit_is_clean());
        assert!(!summary.migration_commit_is_clean());
        assert!(!summary.can_commit_tier_migration());
    }

    #[test]
    fn tier_parser_accepts_adapter_friendly_names() {
        assert_eq!("hot-gpu".parse::<MemoryTier>(), Ok(MemoryTier::HotGpu));
        assert_eq!("ram".parse::<MemoryTier>(), Ok(MemoryTier::WarmRam));
        assert_eq!("disk".parse::<MemoryTier>(), Ok(MemoryTier::ColdDisk));
        assert!("missing".parse::<MemoryTier>().is_err());
    }

    fn placement(id: u64, tier: MemoryTier) -> MemoryPlacement {
        MemoryPlacement {
            id,
            tier,
            score: id as f32,
            reason: format!("placement {id}"),
        }
    }

    fn action_for(migrations: &[TierMigration], id: u64) -> TierMigrationAction {
        migrations
            .iter()
            .find(|migration| migration.id == id)
            .map(|migration| migration.action)
            .expect("migration should exist")
    }
}
