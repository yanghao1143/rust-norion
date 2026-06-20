use rust_norion::DeviceClass;

pub(crate) fn parse_usize(value: &str, fallback: usize) -> usize {
    value.parse::<usize>().unwrap_or(fallback)
}

pub(crate) fn parse_u128(value: &str, fallback: u128) -> u128 {
    value.parse::<u128>().unwrap_or(fallback)
}

pub(crate) fn parse_u64(value: &str, fallback: u64) -> u64 {
    value.parse::<u64>().unwrap_or(fallback)
}

pub(crate) fn parse_f32(value: &str, fallback: f32) -> f32 {
    value.parse::<f32>().unwrap_or(fallback)
}

pub(crate) fn parse_device_or_generic(value: &str) -> DeviceClass {
    value.parse::<DeviceClass>().unwrap_or(DeviceClass::CpuOnly)
}
