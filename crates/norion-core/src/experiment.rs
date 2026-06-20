#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExperimentSwitches {
    pub enable_fht_dke: bool,
    pub enable_adaptive_attention_thresholds: bool,
    pub enable_reinforced_kv_fusion: bool,
    pub enable_runtime_device_abi: bool,
    pub max_attention_tokens: usize,
    pub max_kv_fusion_candidates: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExperimentSwitchesSummary {
    pub enabled_count: usize,
    pub conservative: bool,
    pub fht_dke_enabled: bool,
    pub adaptive_attention_thresholds_enabled: bool,
    pub reinforced_kv_fusion_enabled: bool,
    pub runtime_device_abi_enabled: bool,
    pub max_attention_tokens: usize,
    pub max_kv_fusion_candidates: usize,
}

impl ExperimentSwitchesSummary {
    pub fn has_experimental_features(self) -> bool {
        self.enabled_count > 0
    }

    pub fn has_runtime_planning_features(self) -> bool {
        self.fht_dke_enabled || self.runtime_device_abi_enabled
    }

    pub fn has_attention_or_kv_features(self) -> bool {
        self.adaptive_attention_thresholds_enabled || self.reinforced_kv_fusion_enabled
    }

    pub fn attention_budget_is_conservative(self) -> bool {
        self.max_attention_tokens <= ExperimentSwitches::default().max_attention_tokens
    }

    pub fn kv_fusion_budget_is_conservative(self) -> bool {
        self.max_kv_fusion_candidates <= ExperimentSwitches::default().max_kv_fusion_candidates
    }

    pub fn budgets_are_conservative(self) -> bool {
        self.attention_budget_is_conservative() && self.kv_fusion_budget_is_conservative()
    }
}

impl ExperimentSwitches {
    pub fn conservative() -> Self {
        Self::default()
    }

    pub fn with_fht_dke(mut self, enabled: bool) -> Self {
        self.enable_fht_dke = enabled;
        self
    }

    pub fn with_adaptive_attention_thresholds(mut self, enabled: bool) -> Self {
        self.enable_adaptive_attention_thresholds = enabled;
        self
    }

    pub fn with_reinforced_kv_fusion(mut self, enabled: bool) -> Self {
        self.enable_reinforced_kv_fusion = enabled;
        self
    }

    pub fn with_runtime_device_abi(mut self, enabled: bool) -> Self {
        self.enable_runtime_device_abi = enabled;
        self
    }

    pub fn enabled_count(self) -> usize {
        [
            self.enable_fht_dke,
            self.enable_adaptive_attention_thresholds,
            self.enable_reinforced_kv_fusion,
            self.enable_runtime_device_abi,
        ]
        .into_iter()
        .filter(|enabled| *enabled)
        .count()
    }

    pub fn is_conservative(self) -> bool {
        self.enabled_count() == 0
    }

    pub fn enabled_labels(self) -> Vec<&'static str> {
        let mut labels = Vec::new();
        if self.enable_fht_dke {
            labels.push("fht-dke");
        }
        if self.enable_adaptive_attention_thresholds {
            labels.push("adaptive-attention-thresholds");
        }
        if self.enable_reinforced_kv_fusion {
            labels.push("reinforced-kv-fusion");
        }
        if self.enable_runtime_device_abi {
            labels.push("runtime-device-abi");
        }
        labels
    }

    pub fn summary(self) -> String {
        let labels = self.enabled_labels();
        let enabled = if labels.is_empty() {
            "none".to_owned()
        } else {
            labels.join("+")
        };

        format!(
            "enabled={} max_attention_tokens={} max_kv_fusion_candidates={}",
            enabled, self.max_attention_tokens, self.max_kv_fusion_candidates
        )
    }

    pub fn switches_summary(self) -> ExperimentSwitchesSummary {
        ExperimentSwitchesSummary {
            enabled_count: self.enabled_count(),
            conservative: self.is_conservative(),
            fht_dke_enabled: self.enable_fht_dke,
            adaptive_attention_thresholds_enabled: self.enable_adaptive_attention_thresholds,
            reinforced_kv_fusion_enabled: self.enable_reinforced_kv_fusion,
            runtime_device_abi_enabled: self.enable_runtime_device_abi,
            max_attention_tokens: self.max_attention_tokens,
            max_kv_fusion_candidates: self.max_kv_fusion_candidates,
        }
    }
}

impl Default for ExperimentSwitches {
    fn default() -> Self {
        Self {
            enable_fht_dke: false,
            enable_adaptive_attention_thresholds: false,
            enable_reinforced_kv_fusion: false,
            enable_runtime_device_abi: false,
            max_attention_tokens: 128,
            max_kv_fusion_candidates: 64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn experiment_switch_defaults_are_safe() {
        let switches = ExperimentSwitches::default();
        let summary = switches.switches_summary();

        assert!(!switches.enable_fht_dke);
        assert!(!switches.enable_adaptive_attention_thresholds);
        assert!(!switches.enable_reinforced_kv_fusion);
        assert!(!switches.enable_runtime_device_abi);
        assert!(switches.max_attention_tokens <= 128);
        assert!(switches.max_kv_fusion_candidates <= 64);
        assert_eq!(switches.enabled_count(), 0);
        assert!(switches.is_conservative());
        assert!(switches.enabled_labels().is_empty());
        assert!(switches.summary().contains("enabled=none"));
        assert_eq!(summary.enabled_count, 0);
        assert!(summary.conservative);
        assert!(!summary.has_experimental_features());
        assert!(!summary.has_runtime_planning_features());
        assert!(!summary.has_attention_or_kv_features());
        assert!(summary.budgets_are_conservative());
    }

    #[test]
    fn experiment_switch_summary_reports_enabled_features() {
        let switches = ExperimentSwitches::default()
            .with_fht_dke(true)
            .with_reinforced_kv_fusion(true)
            .with_runtime_device_abi(true);
        let summary = switches.switches_summary();

        assert_eq!(switches.enabled_count(), 3);
        assert!(!switches.is_conservative());
        assert_eq!(
            switches.enabled_labels(),
            vec!["fht-dke", "reinforced-kv-fusion", "runtime-device-abi"]
        );
        assert!(switches.summary().contains("fht-dke"));
        assert!(switches.summary().contains("runtime-device-abi"));
        assert_eq!(summary.enabled_count, 3);
        assert!(!summary.conservative);
        assert!(summary.fht_dke_enabled);
        assert!(!summary.adaptive_attention_thresholds_enabled);
        assert!(summary.reinforced_kv_fusion_enabled);
        assert!(summary.runtime_device_abi_enabled);
        assert!(summary.has_experimental_features());
        assert!(summary.has_runtime_planning_features());
        assert!(summary.has_attention_or_kv_features());
        assert!(summary.budgets_are_conservative());
    }

    #[test]
    fn experiment_switch_summary_reports_budget_expansion() {
        let switches = ExperimentSwitches {
            max_attention_tokens: 256,
            max_kv_fusion_candidates: 96,
            ..ExperimentSwitches::default().with_adaptive_attention_thresholds(true)
        };

        let summary = switches.switches_summary();

        assert_eq!(summary.enabled_count, 1);
        assert!(summary.adaptive_attention_thresholds_enabled);
        assert_eq!(summary.max_attention_tokens, 256);
        assert_eq!(summary.max_kv_fusion_candidates, 96);
        assert!(!summary.attention_budget_is_conservative());
        assert!(!summary.kv_fusion_budget_is_conservative());
        assert!(!summary.budgets_are_conservative());
    }
}
