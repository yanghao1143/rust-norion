#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceMemoryMode {
    MinimalDisk,
    TieredDisk,
    UnifiedMemory,
    GpuResident,
    DistributedSharded,
}

impl DeviceMemoryMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MinimalDisk => "minimal-disk",
            Self::TieredDisk => "tiered-disk",
            Self::UnifiedMemory => "unified-memory",
            Self::GpuResident => "gpu-resident",
            Self::DistributedSharded => "distributed-sharded",
        }
    }
}
