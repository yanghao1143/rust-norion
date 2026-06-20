use super::super::DeviceClass;
use super::HardwareProbe;
use super::load::{default_probe_loads, normalize_load};
use super::snapshot::HardwareSnapshot;
use super::token::sanitize_probe_token;

#[derive(Debug, Clone, PartialEq)]
pub struct HardwareProbeReport {
    pub device: DeviceClass,
    pub reason: String,
    pub os: String,
    pub arch: String,
    pub cpu_count: usize,
    pub accelerator_count: usize,
    pub evidence: Vec<String>,
    pub cpu_load: f32,
    pub gpu_load: f32,
    pub ram_load: f32,
    pub disk_load: f32,
}

impl HardwareProbeReport {
    pub fn snapshot(&self) -> HardwareSnapshot {
        HardwareSnapshot::new(
            self.device,
            self.cpu_load,
            self.gpu_load,
            self.ram_load,
            self.disk_load,
        )
    }

    pub fn summary(&self) -> String {
        let evidence = if self.evidence.is_empty() {
            "none".to_owned()
        } else {
            self.evidence.join("+")
        };
        format!(
            "device={} reason={} os={} arch={} cpus={} accelerators={} loads={:.2}/{:.2}/{:.2}/{:.2} evidence={}",
            self.device.as_str(),
            self.reason,
            self.os,
            self.arch,
            self.cpu_count,
            self.accelerator_count,
            self.cpu_load,
            self.gpu_load,
            self.ram_load,
            self.disk_load,
            evidence
        )
    }
}

impl HardwareProbe {
    pub fn report(&self) -> HardwareProbeReport {
        let mut evidence = vec![
            format!("os:{}", sanitize_probe_token(&self.os)),
            format!("arch:{}", sanitize_probe_token(&self.arch)),
            format!("cpus:{}", self.cpu_count),
        ];
        let accelerator_count = self.accelerator_count();
        if accelerator_count > 0 {
            evidence.push(format!("accelerators:{accelerator_count}"));
        }

        let (device, reason) = self.detect_device_with_evidence(accelerator_count, &mut evidence);
        self.load_evidence(&mut evidence);
        self.report_for(device, reason, accelerator_count, evidence)
    }

    pub(super) fn report_for(
        &self,
        device: DeviceClass,
        reason: &'static str,
        accelerator_count: usize,
        evidence: Vec<String>,
    ) -> HardwareProbeReport {
        let defaults = default_probe_loads(device);
        HardwareProbeReport {
            device,
            reason: reason.to_owned(),
            os: self.os.clone(),
            arch: self.arch.clone(),
            cpu_count: self.cpu_count,
            accelerator_count,
            evidence,
            cpu_load: normalize_load(self.load_hint("NOIRON_CPU_LOAD", defaults.cpu)),
            gpu_load: normalize_load(self.load_hint("NOIRON_GPU_LOAD", defaults.gpu)),
            ram_load: normalize_load(self.load_hint("NOIRON_RAM_LOAD", defaults.ram)),
            disk_load: normalize_load(self.load_hint("NOIRON_DISK_LOAD", defaults.disk)),
        }
    }
}
