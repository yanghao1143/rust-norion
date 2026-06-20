use crate::hardware::DeviceClass;

use super::{BenchmarkCaseResult, BenchmarkSummary};

impl BenchmarkSummary {
    pub fn results(&self) -> &[BenchmarkCaseResult] {
        &self.results
    }

    pub fn len(&self) -> usize {
        self.results.len()
    }

    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }

    pub fn total_elapsed_ms(&self) -> u128 {
        self.results.iter().map(|result| result.elapsed_ms).sum()
    }

    pub fn covered_device_profiles(&self) -> Vec<DeviceClass> {
        let mut devices = Vec::new();

        for result in &self.results {
            if result.device != DeviceClass::Auto && !devices.contains(&result.device) {
                devices.push(result.device);
            }
        }

        devices
    }

    pub fn explicit_device_profiles_covered(&self) -> usize {
        DeviceClass::explicit_profiles()
            .iter()
            .filter(|device| self.results.iter().any(|result| result.device == **device))
            .count()
    }

    pub fn missing_explicit_device_profiles(&self) -> Vec<DeviceClass> {
        DeviceClass::explicit_profiles()
            .iter()
            .copied()
            .filter(|device| !self.results.iter().any(|result| result.device == *device))
            .collect()
    }

    pub fn recursive_device_profiles_covered(&self) -> usize {
        DeviceClass::explicit_profiles()
            .iter()
            .filter(|device| {
                self.results
                    .iter()
                    .any(|result| result.device == **device && result.requires_recursion)
            })
            .count()
    }

    pub fn missing_recursive_device_profiles(&self) -> Vec<DeviceClass> {
        DeviceClass::explicit_profiles()
            .iter()
            .copied()
            .filter(|device| {
                !self
                    .results
                    .iter()
                    .any(|result| result.device == *device && result.requires_recursion)
            })
            .collect()
    }

    pub fn devices_csv(&self) -> String {
        let devices = self
            .covered_device_profiles()
            .into_iter()
            .map(DeviceClass::as_str)
            .collect::<Vec<_>>();

        if devices.is_empty() {
            "none".to_owned()
        } else {
            devices.join("+")
        }
    }

    pub fn recursive_devices_csv(&self) -> String {
        let mut devices = Vec::new();

        for result in &self.results {
            if result.requires_recursion
                && result.device != DeviceClass::Auto
                && !devices.contains(&result.device)
            {
                devices.push(result.device);
            }
        }

        if devices.is_empty() {
            "none".to_owned()
        } else {
            devices
                .into_iter()
                .map(DeviceClass::as_str)
                .collect::<Vec<_>>()
                .join("+")
        }
    }

    pub fn average_quality(&self) -> f32 {
        average(self.results.iter().map(|result| result.quality))
    }

    pub fn average_reward(&self) -> f32 {
        average(self.results.iter().map(|result| result.process_reward))
    }

    pub fn average_attention_fraction(&self) -> f32 {
        average(self.results.iter().map(|result| result.attention_fraction))
    }
}

fn average(values: impl Iterator<Item = f32>) -> f32 {
    let mut total = 0.0;
    let mut count = 0;

    for value in values {
        total += value;
        count += 1;
    }

    if count == 0 {
        0.0
    } else {
        total / count as f32
    }
}
