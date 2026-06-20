use super::json_assert::{require_json_bool, require_json_string};

pub(super) fn smoke_contract_json(alignment_ok: &str) -> String {
    format!(
        concat!(
            "{{",
            "\"schema\":\"smartsteam.forge.model_pool_smoke_contract.v1\",",
            "\"read_only\":true,",
            "\"launches_process\":false,",
            "\"sends_prompt\":false,",
            "\"contract_ok\":true,",
            "\"alignment_ok\":{}",
            "}}"
        ),
        alignment_ok
    )
}

pub(super) fn validate_contract_json(
    contract_json: &str,
    smoke_alignment_ok: bool,
) -> Result<(), String> {
    require_json_string(
        contract_json,
        "schema",
        "smartsteam.forge.model_pool_smoke_contract.v1",
        "model pool smoke contract JSON schema",
    )?;
    require_json_bool(
        contract_json,
        "read_only",
        true,
        "model pool smoke contract JSON read_only",
    )?;
    require_json_bool(
        contract_json,
        "launches_process",
        false,
        "model pool smoke contract JSON launches_process",
    )?;
    require_json_bool(
        contract_json,
        "sends_prompt",
        false,
        "model pool smoke contract JSON sends_prompt",
    )?;
    require_json_bool(
        contract_json,
        "contract_ok",
        true,
        "model pool smoke contract JSON contract_ok",
    )?;
    require_json_bool(
        contract_json,
        "alignment_ok",
        smoke_alignment_ok,
        "model pool smoke contract JSON alignment_ok",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_json_projects_machine_readable_smoke_state() {
        let contract = smoke_contract_json("false");

        assert!(contract.contains("\"schema\":\"smartsteam.forge.model_pool_smoke_contract.v1\""));
        assert!(contract.contains("\"read_only\":true"));
        assert!(contract.contains("\"launches_process\":false"));
        assert!(contract.contains("\"sends_prompt\":false"));
        assert!(contract.contains("\"contract_ok\":true"));
        assert!(contract.contains("\"alignment_ok\":false"));
        validate_contract_json(&contract, false).unwrap();
    }

    #[test]
    fn contract_json_validation_rejects_failed_contract_flag() {
        let contract =
            smoke_contract_json("true").replace("\"contract_ok\":true", "\"contract_ok\":false");

        assert!(
            validate_contract_json(&contract, true)
                .unwrap_err()
                .contains("contract JSON contract_ok")
        );
    }

    #[test]
    fn contract_json_validation_rejects_alignment_mismatch() {
        let contract = smoke_contract_json("false");

        assert!(
            validate_contract_json(&contract, true)
                .unwrap_err()
                .contains("contract JSON alignment_ok")
        );
    }
}
