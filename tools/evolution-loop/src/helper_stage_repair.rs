use std::collections::{BTreeMap, BTreeSet};

use norion_eval::{helper_stage_missing_complete_fields, helper_stage_placeholder_fields};

use crate::helper_feedback;
use crate::json::{json_string, json_string_array};

pub(crate) const VALIDATION_COMMAND: &str =
    "cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --no-fail-fast";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HelperStageRepairStatus {
    pub(crate) latest_round: Option<u64>,
    pub(crate) total_role_count: usize,
    pub(crate) proposals: Vec<HelperStageRepairProposal>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HelperStageRepairProposal {
    pub(crate) proposal_id: String,
    pub(crate) role: String,
    pub(crate) missing_role: bool,
    pub(crate) source_round: Option<u64>,
    pub(crate) evidence_id: String,
    pub(crate) status: String,
    pub(crate) missing_fields: Vec<String>,
    pub(crate) placeholder_fields: Vec<String>,
    pub(crate) matched_fields: Vec<String>,
    pub(crate) expected_fields: Vec<String>,
    pub(crate) suggested_action: String,
}

#[cfg(test)]
pub(crate) fn from_latest_contract_fields(
    latest_round: Option<u64>,
    fields_by_role: BTreeMap<String, BTreeMap<String, String>>,
) -> HelperStageRepairStatus {
    from_latest_contract_fields_with_required_roles(latest_round, fields_by_role, &[])
}

pub(crate) fn from_latest_contract_fields_with_required_roles(
    latest_round: Option<u64>,
    fields_by_role: BTreeMap<String, BTreeMap<String, String>>,
    required_latest_roles: &[String],
) -> HelperStageRepairStatus {
    let total_role_count = fields_by_role.len();
    let present_roles = fields_by_role.keys().cloned().collect::<BTreeSet<_>>();
    let mut proposals = fields_by_role
        .into_iter()
        .filter_map(|(role, fields)| proposal_for_role(latest_round, &role, &fields))
        .collect::<Vec<_>>();
    proposals.extend(missing_required_role_proposals(
        latest_round,
        &present_roles,
        required_latest_roles,
    ));

    HelperStageRepairStatus {
        latest_round,
        total_role_count,
        proposals,
    }
}

pub(crate) fn context_text(status: &HelperStageRepairStatus) -> String {
    let roles = status
        .proposals
        .iter()
        .map(|proposal| proposal.role.clone())
        .collect::<Vec<_>>();
    format!(
        "latest_round={} repair_required={} incomplete_roles={} proposal_count={} report_only=true auto_apply=false",
        option_u64_text(status.latest_round),
        status.repair_required(),
        if roles.is_empty() {
            "none".to_owned()
        } else {
            roles.join(",")
        },
        status.proposals.len()
    )
}

pub(crate) fn status_json(status: &HelperStageRepairStatus) -> String {
    status.report_json()
}

impl HelperStageRepairStatus {
    pub(crate) fn repair_required(&self) -> bool {
        !self.proposals.is_empty()
    }

    fn report_json(&self) -> String {
        let proposals = self
            .proposals
            .iter()
            .map(HelperStageRepairProposal::report_json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"schema\":\"helper_stage_repair_status_report_v1\",\"consumer_surface\":\"evolution_loop_report_only_helper_stage_repair_status\",\"read_only\":true,\"report_only\":true,\"status_loaded\":true,\"source\":\"latest_ledger_helper_stage_contract_projection\",\"latest_round\":{},\"total_role_count\":{},\"repair_required\":{},\"incomplete_role_count\":{},\"proposal_count\":{},\"proposals\":[{}],\"side_effects\":{}}}",
            option_u64_json(self.latest_round),
            self.total_role_count,
            self.repair_required(),
            self.proposals.len(),
            self.proposals.len(),
            proposals,
            side_effects_json()
        )
    }
}

impl HelperStageRepairProposal {
    fn report_json(&self) -> String {
        format!(
            "{{\"proposal_id\":{},\"role\":{},\"target_role\":{},\"missing_role\":{},\"source_round\":{},\"evidence_id\":{},\"status\":{},\"missing_fields\":{},\"placeholder_fields\":{},\"matched_fields\":{},\"expected_fields\":{},\"suggested_action\":{},\"validation\":{{\"command\":{},\"source\":\"static_safe_local_check\",\"command_safety\":\"safe\"}},\"admission\":{{\"status\":\"repair_proposal_report_only\",\"candidate_only\":true,\"auto_apply\":false,\"requires_human_or_main_window_acceptance\":true}},\"side_effects\":{}}}",
            json_string(&self.proposal_id),
            json_string(&self.role),
            json_string(&self.role),
            self.missing_role,
            option_u64_json(self.source_round),
            json_string(&self.evidence_id),
            json_string(&self.status),
            json_string_array(&self.missing_fields),
            json_string_array(&self.placeholder_fields),
            json_string_array(&self.matched_fields),
            json_string_array(&self.expected_fields),
            json_string(&self.suggested_action),
            json_string(VALIDATION_COMMAND),
            side_effects_json()
        )
    }
}

fn proposal_for_role(
    latest_round: Option<u64>,
    role: &str,
    fields: &BTreeMap<String, String>,
) -> Option<HelperStageRepairProposal> {
    let expected_fields = helper_feedback::contract_markers(role)
        .iter()
        .map(|marker| (*marker).to_owned())
        .collect::<Vec<_>>();
    let missing_fields = helper_stage_missing_complete_fields(role, fields, &expected_fields);
    let placeholder_fields = helper_stage_placeholder_fields(role, fields);
    if missing_fields.is_empty() && placeholder_fields.is_empty() {
        return None;
    }

    let matched_fields = expected_fields
        .iter()
        .filter(|field| fields.contains_key(field.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let status = repair_status(&missing_fields, &placeholder_fields).to_owned();
    let repair_fields = repair_fields(&missing_fields, &placeholder_fields);

    Some(HelperStageRepairProposal {
        proposal_id: proposal_id(latest_round, role),
        role: role.to_owned(),
        missing_role: false,
        source_round: latest_round,
        evidence_id: evidence_id(latest_round, role),
        status,
        missing_fields,
        placeholder_fields,
        matched_fields,
        expected_fields,
        suggested_action: suggested_action(role, &repair_fields),
    })
}

fn missing_required_role_proposals(
    latest_round: Option<u64>,
    present_roles: &BTreeSet<String>,
    required_latest_roles: &[String],
) -> Vec<HelperStageRepairProposal> {
    let mut seen = BTreeSet::<String>::new();
    required_latest_roles
        .iter()
        .map(|role| role.trim())
        .filter(|role| !role.is_empty())
        .filter(|role| seen.insert((*role).to_owned()))
        .filter(|role| !present_roles.contains(*role))
        .map(|role| missing_required_role_proposal(latest_round, role))
        .collect()
}

fn missing_required_role_proposal(
    latest_round: Option<u64>,
    role: &str,
) -> HelperStageRepairProposal {
    let expected_fields = helper_feedback::contract_markers(role)
        .iter()
        .map(|marker| (*marker).to_owned())
        .collect::<Vec<_>>();

    HelperStageRepairProposal {
        proposal_id: missing_role_proposal_id(latest_round, role),
        role: role.to_owned(),
        missing_role: true,
        source_round: latest_round,
        evidence_id: missing_role_evidence_id(latest_round, role),
        status: "missing_required_role".to_owned(),
        missing_fields: expected_fields.clone(),
        placeholder_fields: Vec::new(),
        matched_fields: Vec::new(),
        suggested_action: missing_role_suggested_action(role, &expected_fields),
        expected_fields,
    }
}

fn repair_status(missing_fields: &[String], placeholder_fields: &[String]) -> &'static str {
    match (missing_fields.is_empty(), placeholder_fields.is_empty()) {
        (false, false) => "missing_and_placeholder_fields",
        (false, true) => "missing_required_fields",
        (true, false) => "placeholder_fields",
        (true, true) => "complete",
    }
}

fn repair_fields(missing_fields: &[String], placeholder_fields: &[String]) -> Vec<String> {
    let mut fields = missing_fields
        .iter()
        .chain(placeholder_fields.iter())
        .cloned()
        .collect::<Vec<_>>();
    fields.sort();
    fields.dedup();
    fields
}

fn suggested_action(role: &str, repair_fields: &[String]) -> String {
    format!(
        "Repair {role} helper contract by providing concrete {} field(s) in the next helper-stage output.",
        repair_fields.join(",")
    )
}

fn missing_role_suggested_action(role: &str, expected_fields: &[String]) -> String {
    format!(
        "Repair {role} helper stage by producing a latest helper-stage output with concrete {} field(s).",
        expected_fields.join(",")
    )
}

fn proposal_id(latest_round: Option<u64>, role: &str) -> String {
    format!(
        "helper-stage-repair-r{}-{}",
        latest_round.unwrap_or_default(),
        sanitize_role(role)
    )
}

fn missing_role_proposal_id(latest_round: Option<u64>, role: &str) -> String {
    format!(
        "helper-stage-repair-r{}-{}-missing-role",
        latest_round.unwrap_or_default(),
        sanitize_role(role)
    )
}

fn evidence_id(latest_round: Option<u64>, role: &str) -> String {
    format!(
        "ledger.round.{}.helper_stage_contract_by_role.{}",
        latest_round.unwrap_or_default(),
        sanitize_role(role)
    )
}

fn missing_role_evidence_id(latest_round: Option<u64>, role: &str) -> String {
    format!(
        "ledger.round.{}.required_latest_helper_stage_roles.{}.missing",
        latest_round.unwrap_or_default(),
        sanitize_role(role)
    )
}

fn sanitize_role(role: &str) -> String {
    role.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

fn option_u64_json(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

fn option_u64_text(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_owned())
}

fn side_effects_json() -> &'static str {
    "{\"applies_code\":false,\"edits_files\":false,\"mutates_ledger\":false,\"mutates_memory_store\":false,\"writes_ndkv\":false,\"starts_daemon\":false,\"stops_daemon\":false,\"touches_remote\":false,\"downloads_model\":false,\"warms_model_cache\":false,\"starts_forge\":false,\"starts_web_lab\":false,\"sends_prompt\":false,\"starts_stream\":false,\"replays_prompt\":false,\"calls_model\":false}"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proposes_repair_for_missing_review_contract_field() {
        let status = from_latest_contract_fields(
            Some(28),
            BTreeMap::from([(
                "review".to_owned(),
                BTreeMap::from([
                    (
                        "risk".to_owned(),
                        "helper output can be incomplete".to_owned(),
                    ),
                    (
                        "change_request".to_owned(),
                        "surface missing helper fields".to_owned(),
                    ),
                ]),
            )]),
        );
        let json = status_json(&status);

        assert!(status.repair_required());
        assert_eq!(status.proposals[0].role, "review");
        assert_eq!(status.proposals[0].missing_fields, vec!["verification"]);
        assert!(json.contains("\"schema\":\"helper_stage_repair_status_report_v1\""));
        assert!(json.contains("\"repair_required\":true"));
        assert!(json.contains("\"status\":\"missing_required_fields\""));
        assert!(json.contains("\"auto_apply\":false"));
        assert!(json.contains("\"calls_model\":false"));
    }

    #[test]
    fn proposes_report_only_repair_for_missing_required_latest_role() {
        let status = from_latest_contract_fields_with_required_roles(
            Some(29),
            BTreeMap::from([(
                "summary".to_owned(),
                BTreeMap::from([
                    ("memory_update".to_owned(), "keep current route".to_owned()),
                    ("next_context".to_owned(), "continue repair pass".to_owned()),
                    (
                        "duplicate_guard".to_owned(),
                        "single worker only".to_owned(),
                    ),
                ]),
            )]),
            &["summary".to_owned(), "review".to_owned()],
        );
        let json = status_json(&status);

        assert!(status.repair_required());
        assert_eq!(status.proposals.len(), 1);
        assert_eq!(status.proposals[0].role, "review");
        assert!(status.proposals[0].missing_role);
        assert_eq!(status.proposals[0].source_round, Some(29));
        assert_eq!(
            status.proposals[0].evidence_id,
            "ledger.round.29.required_latest_helper_stage_roles.review.missing"
        );
        assert_eq!(status.proposals[0].status, "missing_required_role");
        assert_eq!(
            status.proposals[0].missing_fields,
            vec!["risk", "change_request", "verification"]
        );
        assert!(status.proposals[0].matched_fields.is_empty());
        assert!(json.contains("\"target_role\":\"review\""));
        assert!(json.contains("\"missing_role\":true"));
        assert!(json.contains("\"status\":\"missing_required_role\""));
        assert!(json.contains("\"starts_daemon\":false"));
        assert!(json.contains("\"sends_prompt\":false"));
        assert!(json.contains("\"calls_model\":false"));
    }

    #[test]
    fn treats_passing_test_gate_without_failure_kind_as_complete() {
        let status = from_latest_contract_fields(
            Some(28),
            BTreeMap::from([(
                "test-gate".to_owned(),
                BTreeMap::from([
                    ("verdict".to_owned(), "pass".to_owned()),
                    (
                        "validation_command".to_owned(),
                        "cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --no-fail-fast"
                            .to_owned(),
                    ),
                ]),
            )]),
        );

        assert!(!status.repair_required());
        assert!(status.proposals.is_empty());
    }
}
