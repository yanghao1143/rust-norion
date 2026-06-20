use crate::gemma_business::GemmaModelServiceBusinessCase;
use crate::model_service::json::service_json_string;
use crate::model_service::types::profile_name;

pub(super) fn business_case_request_fields(
    business_case: &GemmaModelServiceBusinessCase,
) -> String {
    format!(
        "\"prompt\":{},\"profile\":\"{}\",\"case\":{}",
        service_json_string(business_case.prompt),
        profile_name(business_case.profile),
        service_json_string(business_case.name)
    )
}

pub(super) fn experience_id_field(experience_id: Option<u64>) -> String {
    format!("\"experience_id\":{}", experience_id.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::{business_case_request_fields, experience_id_field};
    use crate::gemma_business::gemma_business_smoke_case;
    use crate::model_service::json::{json_string_field, json_u64_field};

    #[test]
    fn business_case_request_fields_quote_prompt_and_case_name() {
        let business_case = gemma_business_smoke_case();
        let fields = business_case_request_fields(&business_case);

        assert_eq!(
            json_string_field(&fields, "prompt").as_deref(),
            Some(business_case.prompt)
        );
        assert_eq!(
            json_string_field(&fields, "profile").as_deref(),
            Some("coding")
        );
        assert_eq!(
            json_string_field(&fields, "case").as_deref(),
            Some("gemma-business-runtime")
        );
    }

    #[test]
    fn experience_id_field_defaults_missing_ids_to_zero() {
        assert_eq!(
            json_u64_field(&experience_id_field(Some(7)), "experience_id"),
            Some(7)
        );
        assert_eq!(
            json_u64_field(&experience_id_field(None), "experience_id"),
            Some(0)
        );
    }
}
