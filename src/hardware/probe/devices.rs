use super::HardwareProbe;

const ACCELERATOR_DEVICE_KEYS: &[&str] = &[
    "NOIRON_ACCELERATOR_DEVICES",
    "NOIRON_GPU_DEVICES",
    "CUDA_VISIBLE_DEVICES",
    "NVIDIA_VISIBLE_DEVICES",
    "HIP_VISIBLE_DEVICES",
    "ROCR_VISIBLE_DEVICES",
    "HSA_VISIBLE_DEVICES",
    "GPU_VISIBLE_DEVICES",
    "VULKAN_VISIBLE_DEVICES",
    "DIRECTML_VISIBLE_DEVICES",
    "METAL_VISIBLE_DEVICES",
    "GPU_DEVICE_ORDINAL",
    "ONEAPI_DEVICE_SELECTOR",
    "ZE_AFFINITY_MASK",
    "SYCL_DEVICE_FILTER",
    "ASCEND_RT_VISIBLE_DEVICES",
    "ASCEND_VISIBLE_DEVICES",
    "MLU_VISIBLE_DEVICES",
];

impl HardwareProbe {
    pub(super) fn accelerator_count(&self) -> usize {
        self.env_value_any(ACCELERATOR_DEVICE_KEYS)
            .map(count_visible_devices)
            .unwrap_or(0)
    }
}

fn count_visible_devices(value: &str) -> usize {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || matches!(
            trimmed.to_ascii_lowercase().as_str(),
            "none" | "void" | "disabled" | "-1"
        )
    {
        return 0;
    }
    if trimmed.eq_ignore_ascii_case("all") {
        return 1;
    }

    trimmed
        .split([',', ';'])
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .filter(|item| *item != "-1")
        .count()
}
