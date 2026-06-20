#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeAdapterHint {
    PortableRust,
    CpuSimd,
    Wgpu,
    WebGpu,
    Vulkan,
    Metal,
    Cuda,
    Rocm,
    OneApi,
    DirectMl,
    CoreMl,
    Nnapi,
    Qnn,
    OpenVino,
    Cann,
    Mlu,
    Rknn,
    MultiDevice,
    CustomAccelerator,
}

impl RuntimeAdapterHint {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PortableRust => "portable-rust",
            Self::CpuSimd => "cpu-simd",
            Self::Wgpu => "wgpu",
            Self::WebGpu => "webgpu",
            Self::Vulkan => "vulkan",
            Self::Metal => "metal",
            Self::Cuda => "cuda",
            Self::Rocm => "rocm",
            Self::OneApi => "oneapi",
            Self::DirectMl => "directml",
            Self::CoreMl => "coreml",
            Self::Nnapi => "nnapi",
            Self::Qnn => "qnn",
            Self::OpenVino => "openvino",
            Self::Cann => "cann",
            Self::Mlu => "mlu",
            Self::Rknn => "rknn",
            Self::MultiDevice => "multi-device",
            Self::CustomAccelerator => "custom-accelerator",
        }
    }
}
