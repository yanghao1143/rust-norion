use crate::gemma_business::request_json::experience_id_field;

pub(super) fn feedback_request_body(experience_id: Option<u64>) -> String {
    format!(
        "{{{},\"action\":\"reinforce\",\"amount\":0.5}}",
        experience_id_field(experience_id)
    )
}
