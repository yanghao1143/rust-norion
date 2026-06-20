use super::{DeviceClass, DeviceTier};

#[derive(Debug, Clone, Copy)]
pub struct DeviceProfileDescriptor {
    pub device: DeviceClass,
    pub tier: DeviceTier,
    pub scope: &'static str,
    pub aliases: &'static [&'static str],
}

impl DeviceProfileDescriptor {
    pub fn aliases_csv(&self) -> String {
        self.aliases.join("+")
    }
}
