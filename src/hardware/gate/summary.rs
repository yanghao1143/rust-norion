#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KvPrecisionPolicySummary {
    pub profiles: usize,
    pub hot_q4_profiles: usize,
    pub hot_q8_profiles: usize,
    pub cold_q4_profiles: usize,
    pub runtime_covered_profiles: usize,
    pub order_valid_profiles: usize,
}

impl KvPrecisionPolicySummary {
    pub fn summary_line(self) -> String {
        format!(
            "profiles={} hot_q4={} hot_q8={} cold_q4={} runtime_covered={} order_valid={}",
            self.profiles,
            self.hot_q4_profiles,
            self.hot_q8_profiles,
            self.cold_q4_profiles,
            self.runtime_covered_profiles,
            self.order_valid_profiles
        )
    }
}
