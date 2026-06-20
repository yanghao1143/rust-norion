use std::collections::{BTreeMap, BTreeSet};

use crate::{
    KvEvictionPlan, KvPrefetchPlan, KvShardMetadata, KvTier, MemoryAdapter,
    MemoryAdapterCapability, MemoryAdapterDescriptor, MemoryAdapterHealth, MemoryResult, clamp01,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

    fn rank(self) -> u8 {
        match self {
            Self::HotGpu => 0,
            Self::WarmRam => 1,
            Self::ColdDisk => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryPlacementCandidate {
    pub id: String,
    pub byte_len: usize,
    pub priority: f32,
    pub last_access: u64,
    pub current_tier: Option<MemoryTier>,
}

impl MemoryPlacementCandidate {
    pub fn new(id: impl Into<String>, byte_len: usize) -> Self {
        Self {
            id: id.into(),
            byte_len,
            priority: 0.5,
            last_access: 0,
            current_tier: None,
        }
    }

    pub fn from_kv_metadata(metadata: &KvShardMetadata) -> Self {
        Self {
            id: metadata.id.clone(),
            byte_len: metadata.byte_len,
            priority: metadata.priority,
            last_access: metadata.last_access,
            current_tier: Some(match metadata.tier {
                KvTier::Hot => MemoryTier::WarmRam,
                KvTier::Cold => MemoryTier::ColdDisk,
            }),
        }
    }

    pub fn with_priority(mut self, priority: f32) -> Self {
        self.priority = clamp01(priority);
        self
    }

    pub fn with_last_access(mut self, last_access: u64) -> Self {
        self.last_access = last_access;
        self
    }

    pub fn with_current_tier(mut self, tier: MemoryTier) -> Self {
        self.current_tier = Some(tier);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TierBudgets {
    pub hot_gpu_bytes: usize,
    pub warm_ram_bytes: usize,
}

impl TierBudgets {
    pub fn new(hot_gpu_bytes: usize, warm_ram_bytes: usize) -> Self {
        Self {
            hot_gpu_bytes,
            warm_ram_bytes,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryPlacement {
    pub id: String,
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

impl TierMigrationAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Promote => "promote",
            Self::Demote => "demote",
            Self::Retain => "retain",
            Self::Evict => "evict",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TierMigration {
    pub id: String,
    pub from: Option<MemoryTier>,
    pub to: Option<MemoryTier>,
    pub action: TierMigrationAction,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KvSwapIntent {
    pub prefetch: KvPrefetchPlan,
    pub evict: KvEvictionPlan,
}

impl Default for KvSwapIntent {
    fn default() -> Self {
        Self {
            prefetch: KvPrefetchPlan {
                promote_ids: Vec::new(),
                missing_ids: Vec::new(),
                already_hot_ids: Vec::new(),
                duplicate_ids: Vec::new(),
                reason: String::new(),
            },
            evict: KvEvictionPlan {
                demote_ids: Vec::new(),
                keep_hot_ids: Vec::new(),
                target_hot_bytes: 0,
                reason: String::new(),
            },
        }
    }
}

impl KvSwapIntent {
    pub fn from_migrations(migrations: &[TierMigration], target_hot_bytes: usize) -> Self {
        let promote_ids = migrations
            .iter()
            .filter(|migration| migration.action == TierMigrationAction::Promote)
            .filter(|migration| {
                migration
                    .to
                    .is_some_and(|tier| tier != MemoryTier::ColdDisk)
            })
            .map(|migration| migration.id.clone())
            .collect::<Vec<_>>();
        let demote_ids = migrations
            .iter()
            .filter(|migration| {
                migration.action == TierMigrationAction::Demote
                    && migration.to == Some(MemoryTier::ColdDisk)
            })
            .map(|migration| migration.id.clone())
            .collect::<Vec<_>>();

        Self {
            prefetch: KvPrefetchPlan {
                promote_ids,
                missing_ids: Vec::new(),
                already_hot_ids: Vec::new(),
                duplicate_ids: Vec::new(),
                reason: "tiered_memory_promotions".to_owned(),
            },
            evict: KvEvictionPlan {
                demote_ids,
                keep_hot_ids: Vec::new(),
                target_hot_bytes,
                reason: "tiered_memory_demotions".to_owned(),
            },
        }
    }

    pub fn is_empty(&self) -> bool {
        self.prefetch.promote_ids.is_empty() && self.evict.demote_ids.is_empty()
    }

    pub fn reason_codes(&self) -> Vec<String> {
        let mut codes = BTreeSet::new();
        if !self.prefetch.reason.is_empty() {
            codes.insert(normalized_code(&self.prefetch.reason));
        }
        if !self.evict.reason.is_empty() {
            codes.insert(normalized_code(&self.evict.reason));
        }
        if !self.prefetch.promote_ids.is_empty() {
            codes.insert("prefetch_promote".to_owned());
        }
        if !self.prefetch.missing_ids.is_empty() {
            codes.insert("prefetch_missing".to_owned());
        }
        if !self.evict.demote_ids.is_empty() {
            codes.insert("evict_demote".to_owned());
        }
        if !self.evict.keep_hot_ids.is_empty() {
            codes.insert("evict_keep_hot".to_owned());
        }
        codes.into_iter().collect()
    }

    pub fn detail_codes(&self) -> Vec<String> {
        self.prefetch
            .detail_codes()
            .into_iter()
            .map(|code| format!("prefetch:{code}"))
            .chain(
                self.evict
                    .detail_codes()
                    .into_iter()
                    .map(|code| format!("eviction:{code}")),
            )
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "kvswap_intent empty={} prefetch_promote={} prefetch_missing={} evict_demote={} evict_keep_hot={} target_hot_bytes={} reason_codes={}",
            self.is_empty(),
            self.prefetch.promote_count(),
            self.prefetch.missing_count(),
            self.evict.demote_count(),
            self.evict.keep_hot_count(),
            self.evict.target_hot_bytes,
            join_codes(self.reason_codes()),
        )
    }

    pub fn summary_lines(&self) -> Vec<String> {
        vec![
            self.summary_line(),
            self.prefetch.summary_line(),
            self.evict.summary_line(),
        ]
    }
}

fn normalized_code(value: &str) -> String {
    value
        .split_once('=')
        .map_or(value, |(code, _)| code)
        .to_owned()
}

fn join_codes(codes: Vec<String>) -> String {
    if codes.is_empty() {
        "none".to_owned()
    } else {
        codes.join("|")
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TierCounts {
    pub hot_gpu: usize,
    pub warm_ram: usize,
    pub cold_disk: usize,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TieredMemoryPlan {
    placements: Vec<MemoryPlacement>,
}

impl TieredMemoryPlan {
    pub fn new(placements: Vec<MemoryPlacement>) -> Self {
        Self { placements }
    }

    pub fn placements(&self) -> &[MemoryPlacement] {
        &self.placements
    }

    pub fn placement_for(&self, id: &str) -> Option<&MemoryPlacement> {
        self.placements.iter().find(|placement| placement.id == id)
    }

    pub fn migrations_from(&self, previous: &TieredMemoryPlan) -> Vec<TierMigration> {
        let previous_by_id = previous
            .placements
            .iter()
            .map(|placement| (placement.id.as_str(), placement))
            .collect::<BTreeMap<_, _>>();
        let current_by_id = self
            .placements
            .iter()
            .map(|placement| (placement.id.as_str(), placement))
            .collect::<BTreeMap<_, _>>();
        let mut migrations = Vec::new();

        for current in &self.placements {
            let Some(previous) = previous_by_id.get(current.id.as_str()) else {
                migrations.push(TierMigration {
                    id: current.id.clone(),
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
                id: current.id.clone(),
                from: Some(previous.tier),
                to: Some(current.tier),
                action,
                reason: format!("{} -> {}", previous.reason, current.reason),
            });
        }

        for previous in &previous.placements {
            if !current_by_id.contains_key(previous.id.as_str()) {
                migrations.push(TierMigration {
                    id: previous.id.clone(),
                    from: Some(previous.tier),
                    to: None,
                    action: TierMigrationAction::Evict,
                    reason: format!("evict:{}", previous.reason),
                });
            }
        }

        migrations.sort_by(|left, right| {
            migration_rank(left.action)
                .cmp(&migration_rank(right.action))
                .then_with(|| left.id.cmp(&right.id))
        });
        migrations
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
}

pub trait TieredMemoryPlanner {
    fn plan(
        &self,
        candidates: &[MemoryPlacementCandidate],
        budgets: TierBudgets,
    ) -> TieredMemoryPlan;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DefaultTieredMemoryPlanner;

impl MemoryAdapter for DefaultTieredMemoryPlanner {
    fn descriptor(&self) -> MemoryAdapterDescriptor {
        MemoryAdapterDescriptor::new(
            "default_tiered_memory_planner",
            vec![MemoryAdapterCapability::TieredPlacement],
        )
        .read_only()
    }

    fn health(&self) -> MemoryResult<MemoryAdapterHealth> {
        Ok(MemoryAdapterHealth::ready(None))
    }
}

impl TieredMemoryPlanner for DefaultTieredMemoryPlanner {
    fn plan(
        &self,
        candidates: &[MemoryPlacementCandidate],
        budgets: TierBudgets,
    ) -> TieredMemoryPlan {
        let mut ranked = candidates.to_vec();
        let newest = ranked
            .iter()
            .map(|candidate| candidate.last_access)
            .max()
            .unwrap_or(0);
        ranked.sort_by(|left, right| {
            placement_score(right, newest)
                .partial_cmp(&placement_score(left, newest))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.id.cmp(&right.id))
        });

        let mut hot_used = 0usize;
        let mut warm_used = 0usize;
        let mut placements = Vec::with_capacity(ranked.len());
        for candidate in ranked {
            let score = placement_score(&candidate, newest);
            let (tier, reason) =
                if hot_used + candidate.byte_len <= budgets.hot_gpu_bytes && score >= 0.62 {
                    hot_used += candidate.byte_len;
                    (MemoryTier::HotGpu, "fits_hot_gpu_budget")
                } else if warm_used + candidate.byte_len <= budgets.warm_ram_bytes {
                    warm_used += candidate.byte_len;
                    (MemoryTier::WarmRam, "fits_warm_ram_budget")
                } else {
                    (MemoryTier::ColdDisk, "spills_to_cold_disk")
                };
            placements.push(MemoryPlacement {
                id: candidate.id,
                tier,
                score,
                reason: reason.to_owned(),
            });
        }

        placements.sort_by(|left, right| {
            left.tier
                .rank()
                .cmp(&right.tier.rank())
                .then_with(|| {
                    right
                        .score
                        .partial_cmp(&left.score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| left.id.cmp(&right.id))
        });
        TieredMemoryPlan::new(placements)
    }
}

fn placement_score(candidate: &MemoryPlacementCandidate, newest_access: u64) -> f32 {
    let recency = if newest_access == 0 {
        0.0
    } else {
        1.0 - ((newest_access.saturating_sub(candidate.last_access) as f32) / 256.0).min(1.0)
    };
    (clamp01(candidate.priority) * 0.72 + recency * 0.28).clamp(0.0, 1.0)
}

fn migration_rank(action: TierMigrationAction) -> u8 {
    match action {
        TierMigrationAction::Promote => 0,
        TierMigrationAction::Demote => 1,
        TierMigrationAction::New => 2,
        TierMigrationAction::Retain => 3,
        TierMigrationAction::Evict => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiered_planner_places_high_value_recent_candidates_in_hot_gpu() {
        let candidates = vec![
            MemoryPlacementCandidate::new("hot", 4)
                .with_priority(0.95)
                .with_last_access(20),
            MemoryPlacementCandidate::new("warm", 4)
                .with_priority(0.55)
                .with_last_access(19),
            MemoryPlacementCandidate::new("cold", 4)
                .with_priority(0.20)
                .with_last_access(1),
        ];

        let plan = DefaultTieredMemoryPlanner.plan(&candidates, TierBudgets::new(4, 4));
        assert_eq!(plan.placement_for("hot").unwrap().tier, MemoryTier::HotGpu);
        assert_eq!(
            plan.placement_for("warm").unwrap().tier,
            MemoryTier::WarmRam
        );
        assert_eq!(
            plan.placement_for("cold").unwrap().tier,
            MemoryTier::ColdDisk
        );
        assert_eq!(
            plan.counts(),
            TierCounts {
                hot_gpu: 1,
                warm_ram: 1,
                cold_disk: 1
            }
        );
    }

    #[test]
    fn tiered_plan_reports_promote_demote_retain_and_evict() {
        let previous = TieredMemoryPlan::new(vec![
            MemoryPlacement {
                id: "a".to_owned(),
                tier: MemoryTier::WarmRam,
                score: 0.6,
                reason: "old".to_owned(),
            },
            MemoryPlacement {
                id: "b".to_owned(),
                tier: MemoryTier::HotGpu,
                score: 0.9,
                reason: "old".to_owned(),
            },
            MemoryPlacement {
                id: "c".to_owned(),
                tier: MemoryTier::ColdDisk,
                score: 0.1,
                reason: "old".to_owned(),
            },
            MemoryPlacement {
                id: "gone".to_owned(),
                tier: MemoryTier::WarmRam,
                score: 0.2,
                reason: "old".to_owned(),
            },
        ]);
        let current = TieredMemoryPlan::new(vec![
            MemoryPlacement {
                id: "a".to_owned(),
                tier: MemoryTier::HotGpu,
                score: 0.9,
                reason: "new".to_owned(),
            },
            MemoryPlacement {
                id: "b".to_owned(),
                tier: MemoryTier::ColdDisk,
                score: 0.2,
                reason: "new".to_owned(),
            },
            MemoryPlacement {
                id: "c".to_owned(),
                tier: MemoryTier::ColdDisk,
                score: 0.2,
                reason: "new".to_owned(),
            },
            MemoryPlacement {
                id: "fresh".to_owned(),
                tier: MemoryTier::WarmRam,
                score: 0.5,
                reason: "new".to_owned(),
            },
        ]);

        let migrations = current.migrations_from(&previous);
        assert!(migrations.iter().any(|migration| {
            migration.id == "a" && migration.action == TierMigrationAction::Promote
        }));
        assert!(migrations.iter().any(|migration| {
            migration.id == "b" && migration.action == TierMigrationAction::Demote
        }));
        assert!(migrations.iter().any(|migration| {
            migration.id == "c" && migration.action == TierMigrationAction::Retain
        }));
        assert!(migrations.iter().any(|migration| {
            migration.id == "fresh" && migration.action == TierMigrationAction::New
        }));
        assert!(migrations.iter().any(|migration| {
            migration.id == "gone" && migration.action == TierMigrationAction::Evict
        }));
    }

    #[test]
    fn kv_metadata_projects_to_tiered_candidate() {
        let metadata = KvShardMetadata {
            id: "runtime-kv".to_owned(),
            byte_len: 128,
            checksum: 7,
            tier: KvTier::Cold,
            priority: 0.8,
            last_access: 42,
        };

        let candidate = MemoryPlacementCandidate::from_kv_metadata(&metadata);
        assert_eq!(candidate.id, "runtime-kv");
        assert_eq!(candidate.current_tier, Some(MemoryTier::ColdDisk));
        assert_eq!(candidate.priority, 0.8);
        assert_eq!(candidate.last_access, 42);
    }

    #[test]
    fn tiered_planner_is_read_only_adapter() {
        let descriptor = DefaultTieredMemoryPlanner.descriptor();
        assert_eq!(descriptor.name, "default_tiered_memory_planner");
        assert!(descriptor.read_only);
        assert!(
            descriptor
                .capabilities
                .contains(&MemoryAdapterCapability::TieredPlacement)
        );
        assert!(DefaultTieredMemoryPlanner.health().unwrap().ready);
    }

    #[test]
    fn kvswap_intent_splits_promotions_and_cold_demotions() {
        let migrations = vec![
            TierMigration {
                id: "task/promote shard".to_owned(),
                from: Some(MemoryTier::ColdDisk),
                to: Some(MemoryTier::WarmRam),
                action: TierMigrationAction::Promote,
                reason: "needed soon".to_owned(),
            },
            TierMigration {
                id: "task/demote shard".to_owned(),
                from: Some(MemoryTier::HotGpu),
                to: Some(MemoryTier::ColdDisk),
                action: TierMigrationAction::Demote,
                reason: "budget pressure".to_owned(),
            },
            TierMigration {
                id: "retain".to_owned(),
                from: Some(MemoryTier::WarmRam),
                to: Some(MemoryTier::WarmRam),
                action: TierMigrationAction::Retain,
                reason: "stable".to_owned(),
            },
        ];

        let intent = KvSwapIntent::from_migrations(&migrations, 1024);
        assert_eq!(
            intent.prefetch.promote_ids,
            vec!["task/promote shard".to_owned()]
        );
        assert_eq!(
            intent.evict.demote_ids,
            vec!["task/demote shard".to_owned()]
        );
        assert_eq!(intent.evict.target_hot_bytes, 1024);
        assert!(!intent.is_empty());
        assert_eq!(
            intent.summary_line(),
            "kvswap_intent empty=false prefetch_promote=1 prefetch_missing=0 evict_demote=1 evict_keep_hot=0 target_hot_bytes=1024 reason_codes=evict_demote|prefetch_promote|tiered_memory_demotions|tiered_memory_promotions"
        );
        assert_eq!(
            intent.reason_codes(),
            vec![
                "evict_demote".to_owned(),
                "prefetch_promote".to_owned(),
                "tiered_memory_demotions".to_owned(),
                "tiered_memory_promotions".to_owned()
            ]
        );
        assert_eq!(
            intent.detail_codes(),
            vec![
                "eviction:demote:tiered_memory_demotions:7461736b2f64656d6f7465207368617264"
                    .to_owned(),
                "prefetch:promote:tiered_memory_promotions:7461736b2f70726f6d6f7465207368617264"
                    .to_owned(),
            ]
        );
        assert_eq!(
            intent.summary_lines(),
            vec![
                "kvswap_intent empty=false prefetch_promote=1 prefetch_missing=0 evict_demote=1 evict_keep_hot=0 target_hot_bytes=1024 reason_codes=evict_demote|prefetch_promote|tiered_memory_demotions|tiered_memory_promotions".to_owned(),
                "kvswap_prefetch promote=1 missing=0 hot=0 duplicate=0 reason=tiered_memory_promotions promote_id_hex=7461736b2f70726f6d6f7465207368617264 missing_id_hex=none hot_id_hex=none duplicate_id_hex=none reason_codes=prefetch_promote|tiered_memory_promotions detail_codes=promote:tiered_memory_promotions:7461736b2f70726f6d6f7465207368617264".to_owned(),
                "kvswap_eviction target_hot_bytes=1024 demote=1 keep_hot=0 reason=tiered_memory_demotions demote_id_hex=7461736b2f64656d6f7465207368617264 keep_hot_id_hex=none reason_codes=evict_demote|tiered_memory_demotions detail_codes=demote:tiered_memory_demotions:7461736b2f64656d6f7465207368617264".to_owned(),
            ]
        );
    }

    #[test]
    fn kvswap_intent_evidence_uses_hex_ids_without_raw_shard_payloads() {
        let promote_secret = "KVSWAP_PROMOTE_SHARD_DO_NOT_LOG";
        let demote_secret = "KVSWAP_DEMOTE_SHARD_DO_NOT_LOG";
        let migrations = vec![
            TierMigration {
                id: promote_secret.to_owned(),
                from: Some(MemoryTier::ColdDisk),
                to: Some(MemoryTier::WarmRam),
                action: TierMigrationAction::Promote,
                reason: "needed soon".to_owned(),
            },
            TierMigration {
                id: demote_secret.to_owned(),
                from: Some(MemoryTier::HotGpu),
                to: Some(MemoryTier::ColdDisk),
                action: TierMigrationAction::Demote,
                reason: "budget pressure".to_owned(),
            },
        ];

        let intent = KvSwapIntent::from_migrations(&migrations, 2048);
        let summary_lines = intent.summary_lines();
        let detail_codes = intent.detail_codes();
        let summary_text = summary_lines.join("\n");
        let detail_text = detail_codes.join("\n");

        assert!(summary_text.contains(
            "promote_id_hex=4b56535741505f50524f4d4f54455f53484152445f444f5f4e4f545f4c4f47"
        ));
        assert!(summary_text.contains(
            "demote_id_hex=4b56535741505f44454d4f54455f53484152445f444f5f4e4f545f4c4f47"
        ));
        assert!(detail_codes.contains(
            &"prefetch:promote:tiered_memory_promotions:4b56535741505f50524f4d4f54455f53484152445f444f5f4e4f545f4c4f47"
                .to_owned()
        ));
        assert!(detail_codes.contains(
            &"eviction:demote:tiered_memory_demotions:4b56535741505f44454d4f54455f53484152445f444f5f4e4f545f4c4f47"
                .to_owned()
        ));
        for forbidden in [promote_secret, demote_secret] {
            assert!(
                !summary_text.contains(forbidden),
                "kvswap summary leaked raw shard id: {forbidden}"
            );
            assert!(
                !detail_text.contains(forbidden),
                "kvswap detail codes leaked raw shard id: {forbidden}"
            );
        }
    }

    #[test]
    fn empty_kvswap_intent_has_stable_noop_summary_lines() {
        let intent = KvSwapIntent::default();

        assert!(intent.is_empty());
        assert_eq!(intent.detail_codes(), Vec::<String>::new());
        assert_eq!(
            intent.summary_lines(),
            vec![
                "kvswap_intent empty=true prefetch_promote=0 prefetch_missing=0 evict_demote=0 evict_keep_hot=0 target_hot_bytes=0 reason_codes=none".to_owned(),
                "kvswap_prefetch promote=0 missing=0 hot=0 duplicate=0 reason= promote_id_hex=none missing_id_hex=none hot_id_hex=none duplicate_id_hex=none reason_codes=none detail_codes=none".to_owned(),
                "kvswap_eviction target_hot_bytes=0 demote=0 keep_hot=0 reason= demote_id_hex=none keep_hot_id_hex=none reason_codes=none detail_codes=none".to_owned(),
            ]
        );
    }
}
