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
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "portable-rust" => Some(Self::PortableRust),
            "cpu-simd" => Some(Self::CpuSimd),
            "wgpu" => Some(Self::Wgpu),
            "webgpu" => Some(Self::WebGpu),
            "vulkan" => Some(Self::Vulkan),
            "metal" => Some(Self::Metal),
            "cuda" => Some(Self::Cuda),
            "rocm" => Some(Self::Rocm),
            "oneapi" => Some(Self::OneApi),
            "directml" => Some(Self::DirectMl),
            "coreml" => Some(Self::CoreMl),
            "nnapi" => Some(Self::Nnapi),
            "qnn" => Some(Self::Qnn),
            "openvino" => Some(Self::OpenVino),
            "cann" => Some(Self::Cann),
            "mlu" => Some(Self::Mlu),
            "rknn" => Some(Self::Rknn),
            "multi-device" => Some(Self::MultiDevice),
            "custom-accelerator" => Some(Self::CustomAccelerator),
            _ => None,
        }
    }

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
