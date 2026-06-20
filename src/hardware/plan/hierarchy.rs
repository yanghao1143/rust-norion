use crate::hierarchy::{HierarchyWeights, TaskProfile};

use super::super::device::DeviceClass;

pub(super) fn adapt_hierarchy(
    mut hierarchy: HierarchyWeights,
    device: DeviceClass,
    profile: TaskProfile,
    pressure: f32,
) -> HierarchyWeights {
    match device {
        DeviceClass::CpuOnly | DeviceClass::Edge | DeviceClass::Mobile => {
            hierarchy.local += 0.08;
            hierarchy.convolution += 0.10 + pressure * 0.12;
            hierarchy.global -= pressure * 0.10;
        }
        DeviceClass::Embedded | DeviceClass::BrowserWasm => {
            hierarchy.local += 0.06;
            hierarchy.convolution += 0.18 + pressure * 0.16;
            hierarchy.global -= pressure * 0.14;
        }
        DeviceClass::Microcontroller => {
            hierarchy.local += 0.04;
            hierarchy.convolution += 0.24 + pressure * 0.18;
            hierarchy.global -= pressure * 0.18;
        }
        DeviceClass::IntegratedGpu | DeviceClass::UnifiedMemory => {
            hierarchy.local += 0.04;
            hierarchy.convolution += pressure * 0.08;
        }
        DeviceClass::NpuAccelerator => {
            hierarchy.local += 0.05;
            hierarchy.convolution += pressure * 0.05;
            hierarchy.global += 0.02 * (1.0 - pressure);
        }
        DeviceClass::DiscreteGpu | DeviceClass::Server | DeviceClass::MultiGpu => {
            hierarchy.global += 0.04 * (1.0 - pressure);
            hierarchy.local += 0.03;
        }
        DeviceClass::Auto => {
            hierarchy.convolution += pressure * 0.06;
        }
    }

    if profile == TaskProfile::LongDocument {
        hierarchy.convolution += 0.06;
    }
    if device == DeviceClass::MultiGpu && pressure < 0.45 {
        hierarchy.global += 0.05;
    }

    hierarchy.normalize();
    hierarchy
}
