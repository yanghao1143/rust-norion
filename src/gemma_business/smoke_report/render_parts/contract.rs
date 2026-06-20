use crate::model_service::json::service_json_string_array;

pub(in crate::gemma_business::smoke_report) struct ContractJson<'a> {
    pub(in crate::gemma_business::smoke_report) passed: bool,
    pub(in crate::gemma_business::smoke_report) required_signals: usize,
    pub(in crate::gemma_business::smoke_report) matched_signals: usize,
    pub(in crate::gemma_business::smoke_report) missing_signals: &'a [String],
    pub(in crate::gemma_business::smoke_report) runtime_model_experiences: bool,
    pub(in crate::gemma_business::smoke_report) protocol_leak: bool,
    pub(in crate::gemma_business::smoke_report) substituted_runtime_model_experiences: bool,
    pub(in crate::gemma_business::smoke_report) evasive_denial: bool,
    pub(in crate::gemma_business::smoke_report) handling_signal: bool,
}

pub(in crate::gemma_business::smoke_report) fn contract_json(input: ContractJson<'_>) -> String {
    format!(
        "{{\"passed\":{},\"required_signals\":{},\"matched_signals\":{},\"missing_signals\":{},\"runtime_model_experiences\":{},\"protocol_leak\":{},\"substituted_runtime_model_experiences\":{},\"evasive_denial\":{},\"handling_signal\":{}}}",
        input.passed,
        input.required_signals,
        input.matched_signals,
        service_json_string_array(input.missing_signals),
        input.runtime_model_experiences,
        input.protocol_leak,
        input.substituted_runtime_model_experiences,
        input.evasive_denial,
        input.handling_signal
    )
}

#[cfg(test)]
mod tests {
    use super::{ContractJson, contract_json};

    #[test]
    fn contract_json_renders_contract_gate_evidence() {
        let missing_signals = vec!["case".to_owned(), "signal".to_owned()];

        let body = contract_json(ContractJson {
            passed: false,
            required_signals: 3,
            matched_signals: 1,
            missing_signals: &missing_signals,
            runtime_model_experiences: true,
            protocol_leak: false,
            substituted_runtime_model_experiences: false,
            evasive_denial: true,
            handling_signal: false,
        });

        assert!(body.contains("\"passed\":false"));
        assert!(body.contains("\"missing_signals\":[\"case\",\"signal\"]"));
        assert!(body.contains("\"evasive_denial\":true"));
    }
}
