use crate::gemma_business::model_service_smoke::evidence::BusinessContractEvidence;

pub(in crate::gemma_business::model_service_smoke::gate) fn require_business_contract_normalization_match(
    evidence: BusinessContractEvidence,
    message: &str,
    failures: &mut Vec<String>,
) {
    if !evidence.normalization_counters_match() {
        failures.push(message.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use crate::gemma_business::model_service_smoke::evidence::BusinessContractEvidence;

    use super::require_business_contract_normalization_match;

    #[test]
    fn require_business_contract_normalization_match_records_mismatch_only() {
        let mut failures = Vec::new();

        require_business_contract_normalization_match(
            BusinessContractEvidence {
                response_normalized: 3,
                sanitized: 1,
                canonical_fallbacks: 2,
                ..BusinessContractEvidence::default()
            },
            "matching",
            &mut failures,
        );
        require_business_contract_normalization_match(
            BusinessContractEvidence {
                response_normalized: 4,
                sanitized: 1,
                canonical_fallbacks: 2,
                ..BusinessContractEvidence::default()
            },
            "mismatch",
            &mut failures,
        );

        assert_eq!(failures, vec!["mismatch".to_owned()]);
    }
}
