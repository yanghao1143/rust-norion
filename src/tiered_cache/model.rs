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

    pub(crate) fn rank(self) -> u8 {
        match self {
            Self::HotGpu => 0,
            Self::WarmRam => 1,
            Self::ColdDisk => 2,
        }
    }
}

impl FromStr for MemoryTier {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "hot_gpu" => Ok(Self::HotGpu),
            "warm_ram" => Ok(Self::WarmRam),
            "cold_disk" => Ok(Self::ColdDisk),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct TierMigration {
    pub id: u64,
    pub from: Option<MemoryTier>,
    pub to: Option<MemoryTier>,
    pub action: TierMigrationAction,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TierCounts {
    pub hot_gpu: usize,
    pub warm_ram: usize,
    pub cold_disk: usize,
}
