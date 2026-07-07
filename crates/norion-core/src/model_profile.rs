#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRouteProfile {
    pub source_index: usize,
    pub role: String,
    pub model_profile_id: String,
    pub inference_backend_id: String,
    pub model_pool_id: String,
    pub capabilities: Vec<String>,
    pub blocked_reasons: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModelProfileRegistry {
    pub profiles: Vec<ModelRouteProfile>,
}

impl ModelRouteProfile {
    pub fn route_allowed(&self) -> bool {
        self.blocked_reasons.is_empty()
    }

    fn matches_skill(&self, skill: &str) -> bool {
        self.role.eq_ignore_ascii_case(skill.trim())
    }

    fn supports_capability(&self, capability: &str) -> bool {
        let capability = capability.trim();
        self.capabilities
            .iter()
            .any(|available| available.eq_ignore_ascii_case(capability))
    }
}

impl ModelProfileRegistry {
    pub fn from_profiles(profiles: Vec<ModelRouteProfile>) -> Self {
        Self { profiles }
    }

    pub fn filter_by_skill_and_capability(
        &self,
        skill: &str,
        capability: &str,
    ) -> Vec<&ModelRouteProfile> {
        self.profiles
            .iter()
            .filter(|profile| {
                profile.route_allowed()
                    && profile.matches_skill(skill)
                    && profile.supports_capability(capability)
            })
            .collect()
    }

    pub fn select_for_roles(
        &self,
        roles: &[String],
        capability: &str,
    ) -> Option<&ModelRouteProfile> {
        roles.iter().find_map(|role| {
            self.filter_by_skill_and_capability(role, capability)
                .into_iter()
                .next()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(source_index: usize, role: &str, model: &str, backend: &str) -> ModelRouteProfile {
        let mut blocked_reasons = Vec::new();
        if backend.trim().is_empty() {
            blocked_reasons.push("missing_inference_backend_id".to_owned());
        }
        ModelRouteProfile {
            source_index,
            role: role.to_owned(),
            model_profile_id: model.to_owned(),
            inference_backend_id: backend.to_owned(),
            model_pool_id: format!("pool.{role}"),
            capabilities: vec!["deterministic".to_owned(), "route-proof".to_owned()],
            blocked_reasons,
        }
    }

    #[test]
    fn registry_filters_deterministic_profiles_by_skill_and_capability() {
        let registry = ModelProfileRegistry::from_profiles(vec![
            profile(0, "summary", "summary-det", "deterministic"),
            profile(1, "review", "review-det", "deterministic"),
            profile(2, "review", "review-blocked", ""),
        ]);

        let summary = registry.filter_by_skill_and_capability("summary", "route-proof");
        let review = registry.filter_by_skill_and_capability("review", "route-proof");
        let selected = registry
            .select_for_roles(&["review".to_owned(), "summary".to_owned()], "route-proof")
            .unwrap();

        assert_eq!(summary.len(), 1);
        assert_eq!(summary[0].model_profile_id, "summary-det");
        assert_eq!(review.len(), 1);
        assert_eq!(selected.source_index, 1);
        assert_eq!(selected.model_profile_id, "review-det");
        assert_eq!(selected.inference_backend_id, "deterministic");
        assert_eq!(selected.model_pool_id, "pool.review");
    }

    #[test]
    fn registry_does_not_select_blocked_profiles() {
        let registry = ModelProfileRegistry::from_profiles(vec![
            profile(0, "review", "review-blocked", ""),
            profile(1, "summary", "summary-det", "deterministic"),
        ]);

        assert!(
            registry
                .filter_by_skill_and_capability("review", "route-proof")
                .is_empty()
        );
        assert_eq!(
            registry.select_for_roles(&["review".to_owned()], "route-proof"),
            None
        );
        assert_eq!(
            registry.profiles[0].blocked_reasons,
            vec!["missing_inference_backend_id".to_owned()]
        );
    }
}
