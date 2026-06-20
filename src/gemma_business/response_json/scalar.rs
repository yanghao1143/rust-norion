mod bool;
mod number;
mod string;

pub(in crate::gemma_business) use bool::{
    response_bool_field, response_ok, response_optional_bool_field,
};
pub(in crate::gemma_business) use number::{
    response_optional_u64_field, response_u64_array_field, response_u64_field,
};
pub(in crate::gemma_business) use string::{response_optional_string_field, response_string_field};

#[cfg(test)]
mod tests {
    use super::{
        response_bool_field, response_ok, response_optional_bool_field,
        response_optional_string_field, response_optional_u64_field, response_string_field,
        response_u64_array_field, response_u64_field,
    };

    #[test]
    fn response_ok_accepts_spaced_and_string_encoded_booleans() {
        assert!(response_ok("{ \"ok\" : true }"));
        assert!(response_ok("{\"ok\":\"true\"}"));
        assert!(!response_ok("{ \"ok\" : false }"));
        assert!(!response_ok("{}"));
    }

    #[test]
    fn response_bool_field_reads_named_service_flags() {
        assert!(response_bool_field("{\"passed\" : \"true\"}", "passed"));
        assert!(!response_bool_field("{\"passed\" : \"false\"}", "passed"));
    }

    #[test]
    fn response_default_fields_read_service_values_without_repeating_defaults() {
        let body = "{\"ids\":[2,3],\"answer\":\"ok\"}";

        assert_eq!(response_u64_field(body, "missing"), 0);
        assert_eq!(response_optional_u64_field(body, "missing"), None);
        assert_eq!(response_u64_array_field(body, "ids"), vec![2, 3]);
        assert_eq!(response_u64_array_field(body, "missing"), Vec::<u64>::new());
        assert_eq!(response_string_field(body, "answer"), "ok");
        assert_eq!(
            response_optional_string_field(body, "answer").as_deref(),
            Some("ok")
        );
        assert_eq!(response_string_field(body, "missing"), "");
        assert_eq!(response_optional_bool_field(body, "missing"), None);
    }
}
