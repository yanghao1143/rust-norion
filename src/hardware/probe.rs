use std::env;
use std::thread;

use super::DeviceClass;

mod detection;
mod devices;
mod env_hints;
mod load;
mod report;
mod snapshot;
mod token;

pub use report::HardwareProbeReport;
pub use snapshot::HardwareSnapshot;

#[derive(Debug, Clone)]
pub struct HardwareProbe {
    os: String,
    arch: String,
    cpu_count: usize,
    env: Vec<(String, String)>,
}

impl HardwareProbe {
    pub fn current() -> Self {
        Self {
            os: env::consts::OS.to_owned(),
            arch: env::consts::ARCH.to_owned(),
            cpu_count: thread::available_parallelism()
                .map(|count| count.get())
                .unwrap_or(1),
            env: env::vars().collect(),
        }
    }

    pub fn new(os: impl Into<String>, arch: impl Into<String>, cpu_count: usize) -> Self {
        Self {
            os: os.into(),
            arch: arch.into(),
            cpu_count: cpu_count.max(1),
            env: Vec::new(),
        }
    }

    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    pub fn snapshot(&self) -> HardwareSnapshot {
        self.report().snapshot()
    }

    pub fn detect_device(&self) -> DeviceClass {
        self.report().device
    }
}
