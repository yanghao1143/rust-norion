use super::manifest_projection::model_pool_manifest_json;
use super::{
    ensure_pool_contract, push_field_line, push_manifest_capacity_policy, push_manifest_workers,
    push_model_pool_advice,
};
use crate::provider::json::json_string_field;

pub(crate) fn model_pool_manifest_summary(body: &str) -> Result<String, String> {
    ensure_pool_contract(body, "model pool manifest")?;
    let mut lines = vec!["SmartSteam model pool manifest".to_owned()];
    push_field_line(
        &mut lines,
        "contract_version",
        json_string_field(body, "contract_version"),
    );
    push_field_line(
        &mut lines,
        "manifest_kind",
        json_string_field(body, "manifest_kind"),
    );
    push_manifest_capacity_policy(&mut lines, body);
    push_model_pool_advice(&mut lines, body);
    push_manifest_workers(&mut lines, body);
    lines.push("section=manifest_json".to_owned());
    lines.push(model_pool_manifest_json(body));
    Ok(lines.join("\n"))
}
