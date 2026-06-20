#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComputeLane {
    CpuPortable,
    CpuVector,
    IntegratedGpu,
    DiscreteGpu,
    UnifiedMemoryGpu,
    NeuralAccelerator,
    MultiAccelerator,
    DiskBackedStreaming,
}

impl ComputeLane {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CpuPortable => "cpu-portable",
            Self::CpuVector => "cpu-vector",
            Self::IntegratedGpu => "integrated-gpu",
            Self::DiscreteGpu => "discrete-gpu",
            Self::UnifiedMemoryGpu => "unified-memory-gpu",
            Self::NeuralAccelerator => "neural-accelerator",
            Self::MultiAccelerator => "multi-accelerator",
            Self::DiskBackedStreaming => "disk-backed-streaming",
        }
    }
}
