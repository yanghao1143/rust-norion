use super::HardwareProbe;
use super::token::sanitize_probe_token;

const LOAD_HINT_KEYS: &[&str] = &[
    "NOIRON_CPU_LOAD",
    "NOIRON_GPU_LOAD",
    "NOIRON_RAM_LOAD",
    "NOIRON_DISK_LOAD",
];

const ADAPTER_HINT_KEYS: &[&str] = &[
    "NOIRON_GPU_ADAPTER",
    "WGPU_ADAPTER_NAME",
    "WEBGPU_ADAPTER_NAME",
    "GPU_DEVICE_NAME",
    "DXGI_ADAPTER_NAME",
    "METAL_DEVICE_NAME",
    "VULKAN_DEVICE_NAME",
    "CUDA_DEVICE_NAME",
    "HIP_DEVICE_NAME",
    "ONEAPI_DEVICE_NAME",
    "COREML_DEVICE_NAME",
    "NNAPI_DEVICE_NAME",
    "QNN_DEVICE_NAME",
];

impl HardwareProbe {
    pub(super) fn load_hint(&self, key: &str, fallback: f32) -> f32 {
        self.env_value(key)
            .and_then(|value| value.parse::<f32>().ok())
            .unwrap_or(fallback)
    }

    pub(super) fn load_evidence(&self, evidence: &mut Vec<String>) {
        for &key in LOAD_HINT_KEYS {
            if self.env_value(key).is_some() {
                evidence.push(format!("load:{key}"));
            }
        }
    }

    pub(super) fn env_value(&self, key: &str) -> Option<&str> {
        self.env
            .iter()
            .find(|(env_key, _)| env_key == key)
            .map(|(_, value)| value.as_str())
    }

    pub(super) fn env_value_any(&self, keys: &[&str]) -> Option<&str> {
        keys.iter().find_map(|key| self.env_value(key))
    }

    fn env_key_any<'a>(&self, keys: &'a [&'a str]) -> Option<&'a str> {
        keys.iter()
            .find(|key| self.env_value(key).is_some())
            .copied()
    }

    pub(super) fn npu_hint_evidence(&self) -> Option<String> {
        if self.env_flag("NOIRON_NPU") {
            return Some("env_flag:NOIRON_NPU".to_owned());
        }
        self.env_key_any(&[
            "QNN_SDK_ROOT",
            "HEXAGON_SDK_ROOT",
            "COREML_ENABLE_NEURAL_ENGINE",
            "ANDROID_NNAPI_DEVICE",
            "NPU_VISIBLE_DEVICES",
            "DIRECTML_NPU_DEVICE",
            "ASCEND_HOME_PATH",
            "ASCEND_TOOLKIT_HOME",
            "ASCEND_RT_VISIBLE_DEVICES",
            "CAMBRICON_HOME",
            "MLU_VISIBLE_DEVICES",
            "KUNLUN_HOME",
            "SOPHGO_SDK_ROOT",
            "RKNN_TOOLKIT2",
            "HAILO_SDK_ROOT",
            "VITIS_AI_HOME",
            "ETHOS_U_HOME",
            "ETHOS_N_HOME",
            "MTK_NEUROPILOT_SDK",
        ])
        .map(|key| format!("env:{key}"))
        .or_else(|| {
            self.adapter_hint_evidence(&[
                "npu",
                "neural",
                "ane",
                "hexagon",
                "qnn",
                "tpu",
                "ascend",
                "cann",
                "cambricon",
                "mlu",
                "kunlun",
                "sophgo",
                "rknn",
                "rockchip",
                "horizon",
                "bpu",
                "hailo",
                "ethos",
                "vitis",
            ])
        })
    }

    pub(super) fn discrete_gpu_hint_evidence(&self) -> Option<String> {
        if self.env_flag("NOIRON_DISCRETE_GPU") {
            return Some("env_flag:NOIRON_DISCRETE_GPU".to_owned());
        }
        self.adapter_hint_evidence(&[
            "nvidia",
            "geforce",
            "rtx",
            "tesla",
            "quadro",
            "cuda",
            "radeon rx",
            "radeon pro",
            "amd gpu",
            "arc",
            "discrete",
            "dgpu",
            "opencl",
            "directml",
        ])
    }

    pub(super) fn edge_hint_evidence(&self) -> Option<String> {
        if self.env_flag("NOIRON_EDGE_DEVICE") {
            return Some("env_flag:NOIRON_EDGE_DEVICE".to_owned());
        }
        self.env_key_any(&[
            "JETSON_MODEL_NAME",
            "NVIDIA_JETSON_MODEL",
            "BALENA_DEVICE_TYPE",
            "RPI_MODEL",
            "ROCKCHIP_SOC",
            "NOIRON_EDGE_CLASS",
        ])
        .map(|key| format!("env:{key}"))
        .or_else(|| {
            self.adapter_hint_evidence(&[
                "jetson",
                "tegra",
                "rk3588",
                "rk356",
                "raspberry",
                "edge",
                "gateway",
                "industrial",
            ])
        })
    }

    pub(super) fn unified_memory_hint_evidence(&self) -> Option<String> {
        if self.env_flag("NOIRON_UNIFIED_MEMORY") {
            return Some("env_flag:NOIRON_UNIFIED_MEMORY".to_owned());
        }
        self.adapter_hint_evidence(&[
            "apple",
            "apple silicon",
            "m1",
            "m2",
            "m3",
            "m4",
            "m5",
            "unified",
            "uma",
        ])
    }

    pub(super) fn integrated_gpu_hint_evidence(&self) -> Option<String> {
        if self.env_flag("NOIRON_INTEGRATED_GPU") {
            return Some("env_flag:NOIRON_INTEGRATED_GPU".to_owned());
        }
        self.adapter_hint_evidence(&[
            "integrated",
            "iris",
            "uhd",
            "intel",
            "xe",
            "apu",
            "steam deck",
            "radeon graphics",
        ])
    }

    fn adapter_hint_evidence(&self, needles: &[&str]) -> Option<String> {
        for &key in ADAPTER_HINT_KEYS {
            if let Some(value) = self.env_value(key) {
                let lower = value.to_ascii_lowercase();
                if let Some(needle) = needles.iter().find(|needle| lower.contains(**needle)) {
                    return Some(format!("adapter:{key}:{}", sanitize_probe_token(needle)));
                }
            }
        }
        None
    }

    fn env_flag(&self, key: &str) -> bool {
        self.env_value(key)
            .map(|value| {
                matches!(
                    value.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
            .unwrap_or(false)
    }
}
