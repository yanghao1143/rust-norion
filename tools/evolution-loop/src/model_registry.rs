use std::collections::{BTreeMap, BTreeSet};
use std::sync::RwLock;

const DEFAULT_MODEL_CONFIG: &str = "\
id=local-summary;provider=local;display_name=Local Summary;skill_tags=summary,review;ctx_window=8192;default_max_tokens=1024;cost_tier=free;latency_tier=low;device_class=local-cpu;backend_id=deterministic;backend_ref=local-summary;supports_streaming=true;supports_cancel=true;supports_local=true
id=remote-rust;provider=newapi;display_name=Remote Rust;skill_tags=rust,review;ctx_window=32768;default_max_tokens=4096;cost_tier=medium;latency_tier=medium;device_class=remote;backend_id=newapi-pool;backend_ref=deepseek-v3;supports_streaming=true;supports_cancel=true;supports_openai_compat=true
";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ModelProfile {
    pub(crate) id: String,
    pub(crate) provider: String,
    pub(crate) display_name: String,
    pub(crate) skill_tags: Vec<String>,
    pub(crate) ctx_window: u64,
    pub(crate) default_max_tokens: u64,
    pub(crate) cost_tier: CostTier,
    pub(crate) latency_tier: LatencyTier,
    pub(crate) device_class: DeviceClass,
    pub(crate) backend_ref: BackendRef,
    pub(crate) capabilities: ModelCapabilities,
    pub(crate) dynamics: ModelDynamics,
    pub(crate) policy: ModelPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BackendRef {
    pub(crate) backend_id: String,
    pub(crate) backend_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelCapabilities {
    pub(crate) supports_streaming: bool,
    pub(crate) supports_cancel: bool,
    pub(crate) supports_kv_export: bool,
    pub(crate) supports_local: bool,
    pub(crate) supports_openai_compat: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ModelDynamics {
    pub(crate) reliability_by_skill: BTreeMap<String, f64>,
    pub(crate) recent_drift: f64,
    pub(crate) health: ModelHealth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum ModelHealth {
    Unknown,
    Healthy,
    Degraded,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelPolicy {
    pub(crate) enabled: bool,
    pub(crate) allow_provider_alias: bool,
    pub(crate) deny_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CostTier {
    Free,
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum LatencyTier {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DeviceClass {
    LocalCpu,
    LocalGpu,
    Remote,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelProfileConfig {
    pub(crate) id: String,
    pub(crate) provider: String,
    pub(crate) display_name: String,
    pub(crate) skill_tags: Vec<String>,
    pub(crate) ctx_window: u64,
    pub(crate) default_max_tokens: u64,
    pub(crate) cost_tier: CostTier,
    pub(crate) latency_tier: LatencyTier,
    pub(crate) device_class: DeviceClass,
    pub(crate) backend_id: String,
    pub(crate) backend_ref: String,
    pub(crate) capabilities: ModelCapabilities,
    pub(crate) allow_provider_alias: bool,
}

#[derive(Debug)]
pub(crate) struct ModelRegistry {
    profiles: RwLock<BTreeMap<String, ModelProfile>>,
}

impl ModelProfile {
    pub(crate) fn from_config(config: ModelProfileConfig) -> Self {
        let mut profile = Self {
            id: normalize_token(&config.id),
            provider: config.provider.trim().to_owned(),
            display_name: config.display_name.trim().to_owned(),
            skill_tags: normalize_unique_tags(config.skill_tags),
            ctx_window: config.ctx_window,
            default_max_tokens: config.default_max_tokens,
            cost_tier: config.cost_tier,
            latency_tier: config.latency_tier,
            device_class: config.device_class,
            backend_ref: BackendRef {
                backend_id: config.backend_id.trim().to_owned(),
                backend_ref: config.backend_ref.trim().to_owned(),
            },
            capabilities: config.capabilities,
            dynamics: ModelDynamics::default(),
            policy: ModelPolicy {
                enabled: true,
                allow_provider_alias: config.allow_provider_alias,
                deny_reason: None,
            },
        };
        profile.apply_policy();
        profile
    }

    pub(crate) fn is_enabled(&self) -> bool {
        self.policy.enabled && self.policy.deny_reason.is_none()
    }

    fn apply_policy(&mut self) {
        if let Some(reason) = denied_model_reason(self) {
            self.policy.enabled = false;
            self.policy.deny_reason = Some(reason);
        }
    }
}

impl Default for ModelCapabilities {
    fn default() -> Self {
        Self {
            supports_streaming: false,
            supports_cancel: false,
            supports_kv_export: false,
            supports_local: false,
            supports_openai_compat: false,
        }
    }
}

impl Default for ModelDynamics {
    fn default() -> Self {
        Self {
            reliability_by_skill: BTreeMap::new(),
            recent_drift: 0.0,
            health: ModelHealth::Unknown,
        }
    }
}

impl ModelRegistry {
    pub(crate) fn new() -> Self {
        Self {
            profiles: RwLock::new(BTreeMap::new()),
        }
    }

    pub(crate) fn from_configs(
        configs: Vec<ModelProfileConfig>,
        allowlist_csv: Option<&str>,
    ) -> Result<Self, String> {
        let allowlist = allowlist_csv.map(parse_allowlist_csv);
        let registry = Self::new();
        for config in configs {
            if let Some(allowlist) = &allowlist {
                let id = normalize_token(&config.id);
                if !allowlist.contains(&id) {
                    continue;
                }
            }
            registry.register(ModelProfile::from_config(config))?;
        }
        Ok(registry)
    }

    pub(crate) fn register(&self, profile: ModelProfile) -> Result<(), String> {
        if profile.id.is_empty() {
            return Err("model profile id must not be empty".to_owned());
        }
        let mut profiles = self
            .profiles
            .write()
            .map_err(|_| "model registry lock poisoned".to_owned())?;
        if profiles.contains_key(&profile.id) {
            return Err(format!("duplicate model profile id: {}", profile.id));
        }
        profiles.insert(profile.id.clone(), profile);
        Ok(())
    }

    pub(crate) fn get(&self, id: &str) -> Option<ModelProfile> {
        self.profiles
            .read()
            .ok()
            .and_then(|profiles| profiles.get(&normalize_token(id)).cloned())
    }

    pub(crate) fn list(&self) -> Vec<ModelProfile> {
        self.profiles
            .read()
            .map(|profiles| profiles.values().cloned().collect())
            .unwrap_or_default()
    }

    pub(crate) fn list_enabled(&self) -> Vec<ModelProfile> {
        self.list()
            .into_iter()
            .filter(ModelProfile::is_enabled)
            .collect()
    }

    pub(crate) fn filter_by_skill_tag(&self, skill_tag: &str) -> Vec<ModelProfile> {
        let skill_tag = normalize_token(skill_tag);
        self.list_enabled()
            .into_iter()
            .filter(|profile| profile.skill_tags.iter().any(|tag| tag == &skill_tag))
            .collect()
    }

    pub(crate) fn render_model_list(&self) -> String {
        let mut lines =
            vec!["id\tprovider\ttags\tcost\tlatency\tdevice\thealth\tenabled\tbackend".to_owned()];
        for profile in self.list() {
            lines.push(format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                profile.id,
                profile.provider,
                profile.skill_tags.join(","),
                profile.cost_tier.as_str(),
                profile.latency_tier.as_str(),
                profile.device_class.as_str(),
                profile.dynamics.health.as_str(),
                profile.is_enabled(),
                profile.backend_ref.backend_id
            ));
        }
        lines.join("\n")
    }
}

pub(crate) fn parse_allowlist_csv(input: &str) -> BTreeSet<String> {
    input
        .split(',')
        .map(normalize_token)
        .filter(|value| !value.is_empty())
        .collect()
}

pub(crate) fn parse_typed_config(input: &str) -> Result<Vec<ModelProfileConfig>, String> {
    let mut configs = Vec::new();
    for (line_index, raw_line) in input.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields = parse_key_value_line(line);
        configs.push(
            config_from_fields(fields).map_err(|error| {
                format!("invalid model config line {}: {error}", line_index + 1)
            })?,
        );
    }
    Ok(configs)
}

pub(crate) fn default_model_registry() -> Result<ModelRegistry, String> {
    ModelRegistry::from_configs(parse_typed_config(DEFAULT_MODEL_CONFIG)?, None)
}

fn config_from_fields(fields: BTreeMap<String, String>) -> Result<ModelProfileConfig, String> {
    let id = required_field(&fields, "id")?;
    let provider = required_field(&fields, "provider")?;
    let display_name = fields
        .get("display_name")
        .cloned()
        .unwrap_or_else(|| id.clone());
    let backend_id = fields
        .get("backend_id")
        .cloned()
        .unwrap_or_else(|| provider.clone());
    let backend_ref = fields
        .get("backend_ref")
        .cloned()
        .unwrap_or_else(|| id.clone());
    Ok(ModelProfileConfig {
        id,
        provider,
        display_name,
        skill_tags: parse_list_field(fields.get("skill_tags").map(String::as_str).unwrap_or("")),
        ctx_window: parse_u64_field(&fields, "ctx_window", 8192)?,
        default_max_tokens: parse_u64_field(&fields, "default_max_tokens", 1024)?,
        cost_tier: CostTier::parse(
            fields
                .get("cost_tier")
                .map(String::as_str)
                .unwrap_or("medium"),
        )?,
        latency_tier: LatencyTier::parse(
            fields
                .get("latency_tier")
                .map(String::as_str)
                .unwrap_or("medium"),
        )?,
        device_class: DeviceClass::parse(
            fields
                .get("device_class")
                .map(String::as_str)
                .unwrap_or("remote"),
        )?,
        backend_id,
        backend_ref,
        capabilities: ModelCapabilities {
            supports_streaming: parse_bool_field(&fields, "supports_streaming", false)?,
            supports_cancel: parse_bool_field(&fields, "supports_cancel", false)?,
            supports_kv_export: parse_bool_field(&fields, "supports_kv_export", false)?,
            supports_local: parse_bool_field(&fields, "supports_local", false)?,
            supports_openai_compat: parse_bool_field(&fields, "supports_openai_compat", false)?,
        },
        allow_provider_alias: parse_bool_field(&fields, "allow_provider_alias", false)?,
    })
}

fn denied_model_reason(profile: &ModelProfile) -> Option<String> {
    let candidates = [
        profile.id.as_str(),
        profile.display_name.as_str(),
        profile.backend_ref.backend_ref.as_str(),
    ];
    if candidates
        .iter()
        .any(|candidate| has_denied_gpt_major(candidate))
    {
        return Some("gpt-major-5-or-newer-denied".to_owned());
    }
    if !profile.policy.allow_provider_alias && is_provider_alias(&profile.provider, &profile.id) {
        return Some("provider-default-alias-denied".to_owned());
    }
    None
}

fn has_denied_gpt_major(value: &str) -> bool {
    let normalized = value
        .to_ascii_lowercase()
        .replace(['_', '.', '/', ':'], "-");
    for marker in ["gpt-", "gpt"] {
        let mut rest = normalized.as_str();
        while let Some(index) = rest.find(marker) {
            let after_marker = &rest[index + marker.len()..];
            let digits = after_marker
                .chars()
                .take_while(|character| character.is_ascii_digit())
                .collect::<String>();
            if digits.parse::<u64>().is_ok_and(|major| major >= 5) {
                return true;
            }
            rest = &after_marker[after_marker.len().min(1)..];
        }
    }
    false
}

fn is_provider_alias(provider: &str, id: &str) -> bool {
    let provider = normalize_token(provider);
    let id = normalize_token(id);
    matches!(provider.as_str(), "openai" | "openai-compatible" | "newapi")
        && matches!(
            id.as_str(),
            "default"
                | "latest"
                | "provider-default"
                | "model-default"
                | "openai-default"
                | "openai-latest"
                | "gpt-latest"
        )
}

fn parse_key_value_line(line: &str) -> BTreeMap<String, String> {
    line.split(';')
        .filter_map(|part| part.split_once('='))
        .map(|(key, value)| (normalize_field_key(key), value.trim().to_owned()))
        .collect()
}

fn required_field(fields: &BTreeMap<String, String>, field: &str) -> Result<String, String> {
    fields
        .get(field)
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("missing {field}"))
}

fn parse_u64_field(
    fields: &BTreeMap<String, String>,
    field: &str,
    default: u64,
) -> Result<u64, String> {
    fields
        .get(field)
        .map(|value| {
            value
                .trim()
                .parse::<u64>()
                .map_err(|_| format!("{field} must be an unsigned integer"))
        })
        .unwrap_or(Ok(default))
}

fn parse_bool_field(
    fields: &BTreeMap<String, String>,
    field: &str,
    default: bool,
) -> Result<bool, String> {
    fields
        .get(field)
        .map(|value| match normalize_token(value).as_str() {
            "true" | "yes" | "1" => Ok(true),
            "false" | "no" | "0" => Ok(false),
            _ => Err(format!("{field} must be true or false")),
        })
        .unwrap_or(Ok(default))
}

fn parse_list_field(input: &str) -> Vec<String> {
    normalize_unique_tags(input.split(',').map(ToOwned::to_owned).collect())
}

fn normalize_unique_tags(tags: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    tags.into_iter()
        .map(|tag| normalize_token(&tag))
        .filter(|tag| !tag.is_empty())
        .filter(|tag| seen.insert(tag.clone()))
        .collect()
}

fn normalize_token(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace('_', "-")
}

fn normalize_field_key(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace('-', "_")
}

impl CostTier {
    fn parse(value: &str) -> Result<Self, String> {
        match normalize_token(value).as_str() {
            "free" => Ok(Self::Free),
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            _ => Err(format!("unknown cost_tier: {value}")),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

impl LatencyTier {
    fn parse(value: &str) -> Result<Self, String> {
        match normalize_token(value).as_str() {
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            _ => Err(format!("unknown latency_tier: {value}")),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

impl DeviceClass {
    fn parse(value: &str) -> Result<Self, String> {
        match normalize_token(value).as_str() {
            "local-cpu" => Ok(Self::LocalCpu),
            "local-gpu" => Ok(Self::LocalGpu),
            "remote" => Ok(Self::Remote),
            _ => Err(format!("unknown device_class: {value}")),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::LocalCpu => "local-cpu",
            Self::LocalGpu => "local-gpu",
            Self::Remote => "remote",
        }
    }
}

impl ModelHealth {
    fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Disabled => "disabled",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(id: &str, tags: &[&str]) -> ModelProfileConfig {
        ModelProfileConfig {
            id: id.to_owned(),
            provider: "local".to_owned(),
            display_name: id.to_owned(),
            skill_tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
            ctx_window: 8192,
            default_max_tokens: 1024,
            cost_tier: CostTier::Low,
            latency_tier: LatencyTier::Low,
            device_class: DeviceClass::LocalCpu,
            backend_id: "deterministic".to_owned(),
            backend_ref: id.to_owned(),
            capabilities: ModelCapabilities {
                supports_streaming: true,
                supports_cancel: true,
                supports_kv_export: false,
                supports_local: true,
                supports_openai_compat: false,
            },
            allow_provider_alias: false,
        }
    }

    #[test]
    fn registers_two_profiles_and_filters_by_skill_tag() {
        let registry = ModelRegistry::from_configs(
            vec![
                config("deterministic-small", &["summary", "review"]),
                config("deterministic-code", &["rust", "review"]),
            ],
            None,
        )
        .unwrap();

        let review = registry.filter_by_skill_tag("review");
        let rust = registry.filter_by_skill_tag("rust");

        assert_eq!(review.len(), 2);
        assert_eq!(
            rust.iter()
                .map(|profile| profile.id.as_str())
                .collect::<Vec<_>>(),
            vec!["deterministic-code"]
        );
        assert!(registry.get("DETERMINISTIC_SMALL").is_some());
    }

    #[test]
    fn denies_gpt_5_family_even_when_allowlisted() {
        let configs = vec![
            ModelProfileConfig {
                provider: "openai".to_owned(),
                display_name: "GPT-5.4 preview".to_owned(),
                backend_ref: "gpt-5.4".to_owned(),
                capabilities: ModelCapabilities {
                    supports_openai_compat: true,
                    ..ModelCapabilities::default()
                },
                ..config("gpt-5.4", &["review"])
            },
            ModelProfileConfig {
                provider: "openai".to_owned(),
                display_name: "GPT-6".to_owned(),
                backend_ref: "gpt-6".to_owned(),
                capabilities: ModelCapabilities {
                    supports_openai_compat: true,
                    ..ModelCapabilities::default()
                },
                ..config("gpt-6", &["review"])
            },
        ];

        let registry = ModelRegistry::from_configs(configs, Some("gpt-5.4,gpt-6")).unwrap();

        assert!(registry.get("gpt-5.4").is_some_and(|profile| {
            !profile.is_enabled()
                && profile.policy.deny_reason.as_deref() == Some("gpt-major-5-or-newer-denied")
        }));
        assert!(registry.get("gpt-6").is_some_and(|profile| {
            !profile.is_enabled()
                && profile.policy.deny_reason.as_deref() == Some("gpt-major-5-or-newer-denied")
        }));
        assert!(registry.filter_by_skill_tag("review").is_empty());
    }

    #[test]
    fn denies_openai_provider_alias_even_when_allowlisted() {
        let registry = ModelRegistry::from_configs(
            vec![ModelProfileConfig {
                provider: "openai".to_owned(),
                display_name: "Provider default".to_owned(),
                backend_ref: "default".to_owned(),
                ..config("default", &["review"])
            }],
            Some("default"),
        )
        .unwrap();

        let profile = registry.get("default").unwrap();
        assert!(!profile.is_enabled());
        assert_eq!(
            profile.policy.deny_reason.as_deref(),
            Some("provider-default-alias-denied")
        );
    }

    #[test]
    fn builds_registry_from_comma_allowlist_and_typed_config() {
        let config_text = "\
id=local-fast;provider=local;display_name=Local Fast;skill_tags=summary,review;ctx_window=4096;default_max_tokens=512;cost_tier=free;latency_tier=low;device_class=local-cpu;backend_id=deterministic;backend_ref=local-fast;supports_local=true
id=remote-code;provider=newapi;display_name=Remote Code;skill_tags=rust,review;ctx_window=32768;default_max_tokens=4096;cost_tier=medium;latency_tier=medium;device_class=remote;backend_id=newapi-pool;backend_ref=deepseek-v3;supports_streaming=true;supports_cancel=true;supports_openai_compat=true
id=skip-me;provider=local;skill_tags=summary;backend_id=deterministic
";
        let configs = parse_typed_config(config_text).unwrap();

        let registry =
            ModelRegistry::from_configs(configs, Some("local-fast, remote-code")).unwrap();

        assert!(registry.get("local-fast").is_some());
        assert!(registry.get("remote-code").is_some());
        assert!(registry.get("skip-me").is_none());
        assert_eq!(registry.filter_by_skill_tag("review").len(), 2);
        assert!(
            registry
                .get("remote-code")
                .is_some_and(|profile| profile.capabilities.supports_openai_compat)
        );
    }

    #[test]
    fn renders_model_list_deterministically() {
        let registry = ModelRegistry::from_configs(
            vec![
                config("z-model", &["review"]),
                config("a-model", &["summary", "review"]),
            ],
            None,
        )
        .unwrap();

        let rendered = registry.render_model_list();

        assert!(rendered.starts_with("id\tprovider\ttags"));
        assert!(rendered.find("a-model").unwrap() < rendered.find("z-model").unwrap());
        assert!(rendered.contains("summary,review"));
    }

    #[test]
    fn default_registry_lists_enabled_safe_models() {
        let registry = default_model_registry().unwrap();
        let enabled = registry.list_enabled();

        assert_eq!(enabled.len(), 2);
        assert!(registry.get("local-summary").is_some());
        assert!(registry.get("remote-rust").is_some());
        assert!(registry.get("default").is_none());
        assert!(enabled.iter().all(|profile| !profile.id.starts_with("gpt")));
    }
}
