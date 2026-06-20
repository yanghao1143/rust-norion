use rust_norion::{DeviceClass, HardwareProbeReport, HardwareSnapshot, RecursiveScheduler};

use crate::cli::state::device_scoped_path;

use super::Args;
use super::values::{parse_device_or_generic, parse_f32, parse_u128};

pub(crate) struct DeviceFlagParse<'a> {
    pub(crate) list_devices: &'a mut bool,
    pub(crate) probe_device: &'a mut bool,
    pub(crate) device_gate: &'a mut bool,
    pub(crate) kv_quant_gate: &'a mut bool,
    pub(crate) kv_quant_max_total_us: &'a mut Option<u128>,
    pub(crate) device: &'a mut DeviceClass,
    pub(crate) device_flag_provided: &'a mut bool,
    pub(crate) cpu_load: &'a mut f32,
    pub(crate) gpu_load: &'a mut f32,
    pub(crate) ram_load: &'a mut f32,
    pub(crate) disk_load: &'a mut f32,
    pub(crate) cpu_load_set: &'a mut bool,
    pub(crate) gpu_load_set: &'a mut bool,
    pub(crate) ram_load_set: &'a mut bool,
    pub(crate) disk_load_set: &'a mut bool,
}

impl DeviceFlagParse<'_> {
    pub(crate) fn parse(&mut self, raw: &[String], index: usize) -> Option<usize> {
        match raw.get(index)?.as_str() {
            "--list-devices" => {
                *self.list_devices = true;
                Some(1)
            }
            "--probe-device" => {
                *self.probe_device = true;
                Some(1)
            }
            "--device-gate" => {
                *self.device_gate = true;
                Some(1)
            }
            "--kv-quant-gate" => {
                *self.kv_quant_gate = true;
                Some(1)
            }
            "--kv-quant-max-total-us" => {
                let max_total_us = raw.get(index + 1)?;
                *self.kv_quant_max_total_us = Some(parse_u128(max_total_us, u128::MAX));
                *self.kv_quant_gate = true;
                Some(2)
            }
            "--device" => {
                let device = raw.get(index + 1)?;
                *self.device = parse_device_or_generic(device);
                *self.device_flag_provided = true;
                Some(2)
            }
            "--cpu-load" => {
                let load = raw.get(index + 1)?;
                *self.cpu_load = parse_f32(load, *self.cpu_load);
                *self.cpu_load_set = true;
                Some(2)
            }
            "--gpu-load" => {
                let load = raw.get(index + 1)?;
                *self.gpu_load = parse_f32(load, *self.gpu_load);
                *self.gpu_load_set = true;
                Some(2)
            }
            "--ram-load" => {
                let load = raw.get(index + 1)?;
                *self.ram_load = parse_f32(load, *self.ram_load);
                *self.ram_load_set = true;
                Some(2)
            }
            "--disk-load" => {
                let load = raw.get(index + 1)?;
                *self.disk_load = parse_f32(load, *self.disk_load);
                *self.disk_load_set = true;
                Some(2)
            }
            _ => None,
        }
    }
}

impl Args {
    pub(crate) fn hardware_snapshot(&self) -> HardwareSnapshot {
        HardwareSnapshot::new(
            self.device,
            self.cpu_load,
            self.gpu_load,
            self.ram_load,
            self.disk_load,
        )
    }

    pub(crate) fn prompt_token_estimate(&self) -> usize {
        RecursiveScheduler::new(
            self.native_window_tokens,
            self.chunk_tokens,
            self.chunk_overlap_tokens,
            self.merge_fan_in,
        )
        .plan(&self.prompt)
        .prompt_tokens
    }

    pub(crate) fn effective_probe_report(&self) -> HardwareProbeReport {
        let snapshot = self.hardware_snapshot();
        if let Some(report) = &self.auto_device_probe {
            let mut report = report.clone();
            report.cpu_load = snapshot.cpu_load;
            report.gpu_load = snapshot.gpu_load;
            report.ram_load = snapshot.ram_load;
            report.disk_load = snapshot.disk_load;
            self.append_cli_load_evidence(&mut report.evidence);
            return report;
        }

        let mut evidence = vec![format!("selected_profile:{}", snapshot.device.as_str())];
        self.append_cli_load_evidence(&mut evidence);
        HardwareProbeReport {
            device: snapshot.device,
            reason: if self.device_flag_provided {
                "manual-device".to_owned()
            } else {
                "default-device".to_owned()
            },
            os: "manual".to_owned(),
            arch: "manual".to_owned(),
            cpu_count: 0,
            accelerator_count: 0,
            evidence,
            cpu_load: snapshot.cpu_load,
            gpu_load: snapshot.gpu_load,
            ram_load: snapshot.ram_load,
            disk_load: snapshot.disk_load,
        }
    }

    pub(crate) fn append_cli_load_evidence(&self, evidence: &mut Vec<String>) {
        for (enabled, key) in [
            (self.cpu_load_override, "NOIRON_CPU_LOAD"),
            (self.gpu_load_override, "NOIRON_GPU_LOAD"),
            (self.ram_load_override, "NOIRON_RAM_LOAD"),
            (self.disk_load_override, "NOIRON_DISK_LOAD"),
        ] {
            if enabled {
                let item = format!("cli_override:{key}");
                if !evidence.iter().any(|existing| existing == &item) {
                    evidence.push(item);
                }
            }
        }
    }

    pub(crate) fn for_inspect_device(&self, device: DeviceClass) -> Self {
        let mut args = self.clone();
        args.device = device;
        args.device_flag_provided = true;
        args.auto_device_probe = None;
        args.memory_path = device_scoped_path(&self.memory_path, device);
        args.experience_path = device_scoped_path(&self.experience_path, device);
        args.adaptive_path = device_scoped_path(&self.adaptive_path, device);
        args
    }

    pub(crate) fn for_roundtrip_device(&self, device: DeviceClass) -> Self {
        let mut args = self.clone();
        args.device = device;
        args.device_flag_provided = true;
        args.auto_device_probe = None;
        args.memory_path = device_scoped_path(&self.memory_path, device);
        args.experience_path = device_scoped_path(&self.experience_path, device);
        args.adaptive_path = device_scoped_path(&self.adaptive_path, device);
        args
    }
}
