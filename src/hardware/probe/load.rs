use super::super::DeviceClass;

#[derive(Debug, Clone, Copy)]
pub(super) struct ProbeLoads {
    pub(super) cpu: f32,
    pub(super) gpu: f32,
    pub(super) ram: f32,
    pub(super) disk: f32,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct PressureWeights {
    pub(super) cpu: f32,
    pub(super) gpu: f32,
    pub(super) ram: f32,
    pub(super) disk: f32,
}

pub(super) fn device_pressure_weights(device: DeviceClass) -> PressureWeights {
    match device {
        DeviceClass::CpuOnly => PressureWeights {
            cpu: 0.46,
            gpu: 0.04,
            ram: 0.32,
            disk: 0.18,
        },
        DeviceClass::IntegratedGpu | DeviceClass::UnifiedMemory => PressureWeights {
            cpu: 0.26,
            gpu: 0.24,
            ram: 0.36,
            disk: 0.14,
        },
        DeviceClass::DiscreteGpu => PressureWeights {
            cpu: 0.18,
            gpu: 0.42,
            ram: 0.26,
            disk: 0.14,
        },
        DeviceClass::Mobile => PressureWeights {
            cpu: 0.28,
            gpu: 0.18,
            ram: 0.42,
            disk: 0.12,
        },
        DeviceClass::Embedded => PressureWeights {
            cpu: 0.42,
            gpu: 0.06,
            ram: 0.40,
            disk: 0.12,
        },
        DeviceClass::BrowserWasm => PressureWeights {
            cpu: 0.30,
            gpu: 0.18,
            ram: 0.44,
            disk: 0.08,
        },
        DeviceClass::Microcontroller => PressureWeights {
            cpu: 0.50,
            gpu: 0.00,
            ram: 0.42,
            disk: 0.08,
        },
        DeviceClass::NpuAccelerator => PressureWeights {
            cpu: 0.18,
            gpu: 0.34,
            ram: 0.36,
            disk: 0.12,
        },
        DeviceClass::MultiGpu => PressureWeights {
            cpu: 0.16,
            gpu: 0.46,
            ram: 0.22,
            disk: 0.16,
        },
        DeviceClass::Edge => PressureWeights {
            cpu: 0.34,
            gpu: 0.12,
            ram: 0.38,
            disk: 0.16,
        },
        DeviceClass::Server => PressureWeights {
            cpu: 0.24,
            gpu: 0.34,
            ram: 0.24,
            disk: 0.18,
        },
        DeviceClass::Auto => PressureWeights {
            cpu: 0.25,
            gpu: 0.25,
            ram: 0.34,
            disk: 0.16,
        },
    }
}

pub(super) fn normalize_load(value: f32) -> f32 {
    if value > 1.0 {
        (value / 100.0).clamp(0.0, 1.0)
    } else {
        value.clamp(0.0, 1.0)
    }
}

pub(super) fn default_probe_loads(device: DeviceClass) -> ProbeLoads {
    match device {
        DeviceClass::Mobile => ProbeLoads {
            cpu: 0.30,
            gpu: 0.20,
            ram: 0.55,
            disk: 0.10,
        },
        DeviceClass::Embedded => ProbeLoads {
            cpu: 0.35,
            gpu: 0.05,
            ram: 0.60,
            disk: 0.15,
        },
        DeviceClass::BrowserWasm => ProbeLoads {
            cpu: 0.30,
            gpu: 0.18,
            ram: 0.62,
            disk: 0.08,
        },
        DeviceClass::Microcontroller => ProbeLoads {
            cpu: 0.45,
            gpu: 0.00,
            ram: 0.72,
            disk: 0.18,
        },
        DeviceClass::Edge => ProbeLoads {
            cpu: 0.32,
            gpu: 0.15,
            ram: 0.48,
            disk: 0.18,
        },
        DeviceClass::NpuAccelerator => ProbeLoads {
            cpu: 0.22,
            gpu: 0.28,
            ram: 0.42,
            disk: 0.12,
        },
        DeviceClass::MultiGpu => ProbeLoads {
            cpu: 0.18,
            gpu: 0.24,
            ram: 0.28,
            disk: 0.12,
        },
        DeviceClass::Server => ProbeLoads {
            cpu: 0.18,
            gpu: 0.22,
            ram: 0.30,
            disk: 0.16,
        },
        _ => ProbeLoads {
            cpu: 0.20,
            gpu: 0.20,
            ram: 0.35,
            disk: 0.15,
        },
    }
}
