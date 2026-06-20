use super::super::DeviceClass;
use super::HardwareProbe;
use super::token::sanitize_probe_token;

impl HardwareProbe {
    pub(super) fn detect_device_with_evidence(
        &self,
        accelerator_count: usize,
        evidence: &mut Vec<String>,
    ) -> (DeviceClass, &'static str) {
        if let Some(value) = self.env_value("NOIRON_DEVICE_PROFILE") {
            let alias = sanitize_probe_token(value);
            evidence.push(format!("explicit_profile_alias:{alias}"));
            match value.parse::<DeviceClass>() {
                Ok(DeviceClass::Auto) => {
                    evidence.push("explicit_profile:auto".to_owned());
                }
                Ok(device) => {
                    evidence.push(format!("explicit_profile:{}", device.as_str()));
                    return (device, "explicit-profile");
                }
                Err(_) => {
                    evidence.push(format!("unknown_explicit_profile:{alias}"));
                    evidence.push("portable_cpu_fallback".to_owned());
                    return (DeviceClass::CpuOnly, "unknown-explicit-profile");
                }
            }
        }

        let os = self.os.to_ascii_lowercase();
        let arch = self.arch.to_ascii_lowercase();

        if matches!(
            os.as_str(),
            "android" | "ios" | "tvos" | "visionos" | "watchos"
        ) {
            evidence.push(format!("mobile_os:{os}"));
            return (DeviceClass::Mobile, "mobile-os");
        }
        if arch.starts_with("wasm") || matches!(os.as_str(), "wasi") {
            evidence.push("wasm_target".to_owned());
            return (DeviceClass::BrowserWasm, "wasm-target");
        }
        if matches!(os.as_str(), "espidf" | "none")
            || arch.contains("xtensa")
            || arch.starts_with("thumb")
            || arch.contains("cortex-m")
            || arch == "riscv32"
        {
            evidence.push("microcontroller_target".to_owned());
            return (DeviceClass::Microcontroller, "microcontroller-target");
        }
        if let Some(hint) = self.npu_hint_evidence() {
            evidence.push(hint);
            return (DeviceClass::NpuAccelerator, "npu-hint");
        }
        if let Some(hint) = self.edge_hint_evidence() {
            evidence.push(hint);
            return (DeviceClass::Edge, "edge-hint");
        }

        if accelerator_count > 1 {
            return (DeviceClass::MultiGpu, "multi-accelerator");
        }
        if accelerator_count == 1 {
            if let Some(hint) = self.unified_memory_hint_evidence() {
                evidence.push(hint);
                return (DeviceClass::UnifiedMemory, "unified-memory-hint");
            }
            if let Some(hint) = self.integrated_gpu_hint_evidence() {
                evidence.push(hint);
                return (DeviceClass::IntegratedGpu, "integrated-gpu-hint");
            }
            return (DeviceClass::DiscreteGpu, "single-accelerator");
        }

        if let Some(hint) = self.unified_memory_hint_evidence() {
            evidence.push(hint);
            return (DeviceClass::UnifiedMemory, "unified-memory-hint");
        }
        if os == "macos" && is_arm_arch(&arch) {
            evidence.push("apple_silicon_default".to_owned());
            return (DeviceClass::UnifiedMemory, "unified-memory-default");
        }
        if let Some(hint) = self.integrated_gpu_hint_evidence() {
            evidence.push(hint);
            return (DeviceClass::IntegratedGpu, "integrated-gpu-hint");
        }
        if let Some(hint) = self.discrete_gpu_hint_evidence() {
            evidence.push(hint);
            return (DeviceClass::DiscreteGpu, "discrete-gpu-hint");
        }
        if os == "linux" && is_arm_arch(&arch) {
            return if self.cpu_count <= 4 {
                evidence.push("linux_arm_embedded".to_owned());
                (DeviceClass::Embedded, "linux-arm-embedded")
            } else {
                evidence.push("linux_arm_edge".to_owned());
                (DeviceClass::Edge, "linux-arm-edge")
            };
        }
        if self.cpu_count >= 32 {
            evidence.push("high_cpu_count".to_owned());
            return (DeviceClass::Server, "high-cpu-count");
        }

        evidence.push("portable_cpu_default".to_owned());
        (DeviceClass::CpuOnly, "portable-cpu-default")
    }
}

fn is_arm_arch(arch: &str) -> bool {
    arch.contains("arm") || arch.contains("aarch64")
}
