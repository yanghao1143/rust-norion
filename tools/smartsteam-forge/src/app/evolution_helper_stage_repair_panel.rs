use super::status_json::{
    bool_value_text, compact_line, json_bool_field, json_number_field, json_object_array_field,
    json_object_field, json_string_array_field, json_string_field, json_string_literal,
    json_top_level_object_field, json_top_level_string_field,
};

pub(super) fn helper_stage_repair_panel_lines(report_json: &str) -> Vec<String> {
    let Some(panel) = HelperStageRepairPanel::from_report_json(Some(report_json)) else {
        return Vec::new();
    };

    let mut lines = vec![format!(
        "helper_stage_repair_panel read_only={} starts_process={} sends_prompt={} status_loaded={} report_only={} safe={} source={} latest_round={} repair_required={} total_roles={} incomplete_roles={} missing_helper_role_repair_required={} missing_helper_role_repair_proposals={} missing_helper_roles={} proposals={} roles={} missing_fields={} placeholder_fields={} starts_daemon={} starts_forge={} starts_web_lab={} calls_model={} starts_stream={} replays_prompt={} writes_ndkv={} mutates_memory_store={} auto_apply={}",
        bool_value_text(panel.read_only),
        bool_value_text(panel.starts_process),
        bool_value_text(panel.sends_prompt),
        bool_value_text(panel.status_loaded),
        bool_value_text(panel.report_only),
        bool_value_text(panel.safe),
        panel.source.as_deref().unwrap_or("unknown"),
        panel.latest_round.as_deref().unwrap_or("unknown"),
        bool_value_text(panel.repair_required),
        panel.total_role_count,
        panel.incomplete_role_count,
        bool_value_text(panel.missing_helper_role_repair_required),
        panel.missing_helper_role_repair_proposal_count,
        list_value(&panel.missing_helper_roles),
        panel.proposal_count,
        list_value(&panel.roles),
        list_value(&panel.missing_fields),
        list_value(&panel.placeholder_fields),
        bool_value_text(panel.side_effects.starts_daemon),
        bool_value_text(panel.side_effects.starts_forge),
        bool_value_text(panel.side_effects.starts_web_lab),
        bool_value_text(panel.side_effects.calls_model),
        bool_value_text(panel.side_effects.starts_stream),
        bool_value_text(panel.side_effects.replays_prompt),
        bool_value_text(panel.side_effects.writes_ndkv),
        bool_value_text(panel.side_effects.mutates_memory_store),
        bool_value_text(panel.auto_apply)
    )];

    lines.extend(panel.proposals.iter().map(HelperStageRepairProposal::line));
    lines
}

pub(super) fn helper_stage_repair_panel_json(report_json: Option<&str>) -> String {
    HelperStageRepairPanel::from_report_json(report_json)
        .unwrap_or_default()
        .to_json()
}

struct HelperStageRepairPanel {
    read_only: bool,
    starts_process: bool,
    sends_prompt: bool,
    status_loaded: bool,
    report_only: bool,
    safe: bool,
    source: Option<String>,
    latest_round: Option<String>,
    repair_required: bool,
    total_role_count: String,
    incomplete_role_count: String,
    missing_helper_role_repair_required: bool,
    missing_helper_role_repair_proposal_count: String,
    missing_helper_roles: Vec<String>,
    proposal_count: String,
    roles: Vec<String>,
    proposal_ids: Vec<String>,
    missing_fields: Vec<String>,
    placeholder_fields: Vec<String>,
    validation_safe: bool,
    candidate_only: bool,
    auto_apply: bool,
    proposals: Vec<HelperStageRepairProposal>,
    side_effects: HelperStageRepairSideEffects,
}

impl Default for HelperStageRepairPanel {
    fn default() -> Self {
        Self {
            read_only: true,
            starts_process: false,
            sends_prompt: false,
            status_loaded: false,
            report_only: true,
            safe: true,
            source: None,
            latest_round: None,
            repair_required: false,
            total_role_count: "0".to_owned(),
            incomplete_role_count: "0".to_owned(),
            missing_helper_role_repair_required: false,
            missing_helper_role_repair_proposal_count: "0".to_owned(),
            missing_helper_roles: Vec::new(),
            proposal_count: "0".to_owned(),
            roles: Vec::new(),
            proposal_ids: Vec::new(),
            missing_fields: Vec::new(),
            placeholder_fields: Vec::new(),
            validation_safe: true,
            candidate_only: true,
            auto_apply: false,
            proposals: Vec::new(),
            side_effects: HelperStageRepairSideEffects::default(),
        }
    }
}

impl HelperStageRepairPanel {
    fn from_report_json(report_json: Option<&str>) -> Option<Self> {
        let report = helper_stage_repair_report(report_json?)?;
        let proposals = json_object_array_field(report, "proposals")
            .unwrap_or_default()
            .into_iter()
            .map(HelperStageRepairProposal::from_json)
            .collect::<Vec<_>>();
        let mut side_effects = HelperStageRepairSideEffects::from_json(
            json_top_level_object_field(report, "side_effects"),
        );
        for proposal in &proposals {
            side_effects.absorb(&proposal.side_effects);
        }

        let mut roles = Vec::new();
        let mut proposal_ids = Vec::new();
        let mut missing_fields = Vec::new();
        let mut placeholder_fields = Vec::new();
        let mut missing_helper_roles = json_string_array_field(report, "missing_helper_roles")
            .unwrap_or_default()
            .into_iter()
            .map(|value| compact_line(&value, 120))
            .collect::<Vec<_>>();
        for proposal in &proposals {
            push_unique(&mut roles, proposal.role.clone());
            push_unique(&mut proposal_ids, proposal.proposal_id.clone());
            push_unique_all(&mut missing_fields, &proposal.missing_fields);
            push_unique_all(&mut placeholder_fields, &proposal.placeholder_fields);
            if proposal.missing_helper_role_repair_required {
                push_unique(&mut missing_helper_roles, proposal.role.clone());
            }
        }

        let read_only = json_bool_field(report, "read_only").unwrap_or(true);
        let starts_process = json_bool_field(report, "starts_process").unwrap_or(false);
        let sends_prompt =
            json_bool_field(report, "sends_prompt").unwrap_or(false) || side_effects.sends_prompt;
        let report_only = json_bool_field(report, "report_only").unwrap_or(true);
        let validation_safe = proposals
            .iter()
            .all(|proposal| proposal.validation_command_safety == "safe");
        let candidate_only = proposals.iter().all(|proposal| proposal.candidate_only);
        let auto_apply = proposals.iter().any(|proposal| proposal.auto_apply);
        let repair_required =
            json_bool_field(report, "repair_required").unwrap_or(!proposals.is_empty());
        let missing_helper_role_repair_proposal_count = number_or_count_from_fields(
            report,
            &[
                "missing_helper_role_repair_proposal_count",
                "missing_helper_role_count",
            ],
            missing_helper_roles.len(),
        );
        let missing_helper_role_repair_required =
            json_bool_field(report, "missing_helper_role_repair_required")
                .unwrap_or_else(|| count_is_nonzero(&missing_helper_role_repair_proposal_count));
        let safe = read_only
            && !starts_process
            && !sends_prompt
            && report_only
            && side_effects.safe()
            && validation_safe
            && candidate_only
            && !auto_apply;

        Some(Self {
            read_only,
            starts_process,
            sends_prompt,
            status_loaded: true,
            report_only,
            safe,
            source: json_string_field(report, "source").map(|value| compact_line(&value, 160)),
            latest_round: json_number_field(report, "latest_round"),
            repair_required,
            total_role_count: number_or_count(report, "total_role_count", roles.len()),
            incomplete_role_count: number_or_count(report, "incomplete_role_count", roles.len()),
            missing_helper_role_repair_required,
            missing_helper_role_repair_proposal_count,
            missing_helper_roles,
            proposal_count: number_or_count(report, "proposal_count", proposals.len()),
            roles,
            proposal_ids,
            missing_fields,
            placeholder_fields,
            validation_safe,
            candidate_only,
            auto_apply,
            proposals,
            side_effects,
        })
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"read_only\":{},\"starts_process\":{},\"sends_prompt\":{},\"status_loaded\":{},\"report_only\":{},\"safe\":{},\"source\":{},\"latest_round\":{},\"repair_required\":{},\"total_role_count\":{},\"incomplete_role_count\":{},\"missing_helper_role_repair_required\":{},\"missing_helper_role_repair_proposal_count\":{},\"missing_helper_roles\":{},\"proposal_count\":{},\"roles\":{},\"proposal_ids\":{},\"missing_fields\":{},\"placeholder_fields\":{},\"validation_safe\":{},\"candidate_only\":{},\"auto_apply\":{},\"proposals\":{},\"side_effects\":{}}}",
            bool_value_text(self.read_only),
            bool_value_text(self.starts_process),
            bool_value_text(self.sends_prompt),
            bool_value_text(self.status_loaded),
            bool_value_text(self.report_only),
            bool_value_text(self.safe),
            optional_string_json(self.source.as_deref()),
            self.latest_round.as_deref().unwrap_or("null"),
            bool_value_text(self.repair_required),
            self.total_role_count,
            self.incomplete_role_count,
            bool_value_text(self.missing_helper_role_repair_required),
            self.missing_helper_role_repair_proposal_count,
            string_array_json(&self.missing_helper_roles),
            self.proposal_count,
            string_array_json(&self.roles),
            string_array_json(&self.proposal_ids),
            string_array_json(&self.missing_fields),
            string_array_json(&self.placeholder_fields),
            bool_value_text(self.validation_safe),
            bool_value_text(self.candidate_only),
            bool_value_text(self.auto_apply),
            proposals_json(&self.proposals),
            self.side_effects.to_json()
        )
    }
}

struct HelperStageRepairProposal {
    proposal_id: String,
    role: String,
    status: String,
    missing_helper_role_repair_required: bool,
    missing_fields: Vec<String>,
    placeholder_fields: Vec<String>,
    validation_command_safety: String,
    candidate_only: bool,
    auto_apply: bool,
    side_effects: HelperStageRepairSideEffects,
}

impl HelperStageRepairProposal {
    fn from_json(proposal: &str) -> Self {
        let validation = json_object_field(proposal, "validation");
        let admission = json_object_field(proposal, "admission");
        let status = compact_string_field(proposal, "status");
        Self {
            proposal_id: compact_string_field(proposal, "proposal_id"),
            role: compact_string_field(proposal, "role"),
            missing_helper_role_repair_required: is_missing_helper_role_status(&status),
            status,
            missing_fields: compact_array_field(proposal, "missing_fields"),
            placeholder_fields: compact_array_field(proposal, "placeholder_fields"),
            validation_command_safety: validation
                .and_then(|value| json_string_field(value, "command_safety"))
                .map(|value| compact_line(&value, 80))
                .unwrap_or_else(|| "unknown".to_owned()),
            candidate_only: admission
                .and_then(|value| json_bool_field(value, "candidate_only"))
                .unwrap_or(true),
            auto_apply: admission
                .and_then(|value| json_bool_field(value, "auto_apply"))
                .unwrap_or(false),
            side_effects: HelperStageRepairSideEffects::from_json(json_object_field(
                proposal,
                "side_effects",
            )),
        }
    }

    fn line(&self) -> String {
        format!(
            "helper_stage_repair_proposal id={} role={} status={} missing_helper_role_repair_required={} missing_fields={} placeholder_fields={} validation_safety={} candidate_only={} auto_apply={} starts_daemon={} calls_model={} starts_stream={} replays_prompt={}",
            self.proposal_id,
            self.role,
            self.status,
            bool_value_text(self.missing_helper_role_repair_required),
            list_value(&self.missing_fields),
            list_value(&self.placeholder_fields),
            self.validation_command_safety,
            bool_value_text(self.candidate_only),
            bool_value_text(self.auto_apply),
            bool_value_text(self.side_effects.starts_daemon),
            bool_value_text(self.side_effects.calls_model),
            bool_value_text(self.side_effects.starts_stream),
            bool_value_text(self.side_effects.replays_prompt)
        )
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"read_only\":true,\"starts_process\":false,\"sends_prompt\":false,\"proposal_id\":{},\"role\":{},\"status\":{},\"missing_helper_role_repair_required\":{},\"missing_fields\":{},\"placeholder_fields\":{},\"validation_command_safety\":{},\"candidate_only\":{},\"auto_apply\":{},\"side_effects\":{}}}",
            json_string_literal(&self.proposal_id),
            json_string_literal(&self.role),
            json_string_literal(&self.status),
            bool_value_text(self.missing_helper_role_repair_required),
            string_array_json(&self.missing_fields),
            string_array_json(&self.placeholder_fields),
            json_string_literal(&self.validation_command_safety),
            bool_value_text(self.candidate_only),
            bool_value_text(self.auto_apply),
            self.side_effects.to_json()
        )
    }
}

#[derive(Clone, Default)]
struct HelperStageRepairSideEffects {
    applies_code: bool,
    edits_files: bool,
    mutates_ledger: bool,
    mutates_memory_store: bool,
    writes_ndkv: bool,
    starts_daemon: bool,
    stops_daemon: bool,
    touches_remote: bool,
    downloads_model: bool,
    warms_model_cache: bool,
    starts_forge: bool,
    starts_web_lab: bool,
    sends_prompt: bool,
    starts_stream: bool,
    replays_prompt: bool,
    calls_model: bool,
}

impl HelperStageRepairSideEffects {
    fn from_json(side_effects: Option<&str>) -> Self {
        Self {
            applies_code: bool_from(side_effects, "applies_code"),
            edits_files: bool_from(side_effects, "edits_files"),
            mutates_ledger: bool_from(side_effects, "mutates_ledger"),
            mutates_memory_store: bool_from(side_effects, "mutates_memory_store"),
            writes_ndkv: bool_from(side_effects, "writes_ndkv"),
            starts_daemon: bool_from(side_effects, "starts_daemon"),
            stops_daemon: bool_from(side_effects, "stops_daemon"),
            touches_remote: bool_from(side_effects, "touches_remote"),
            downloads_model: bool_from(side_effects, "downloads_model"),
            warms_model_cache: bool_from(side_effects, "warms_model_cache"),
            starts_forge: bool_from(side_effects, "starts_forge"),
            starts_web_lab: bool_from(side_effects, "starts_web_lab"),
            sends_prompt: bool_from(side_effects, "sends_prompt"),
            starts_stream: bool_from(side_effects, "starts_stream"),
            replays_prompt: bool_from(side_effects, "replays_prompt"),
            calls_model: bool_from(side_effects, "calls_model"),
        }
    }

    fn absorb(&mut self, other: &Self) {
        self.applies_code |= other.applies_code;
        self.edits_files |= other.edits_files;
        self.mutates_ledger |= other.mutates_ledger;
        self.mutates_memory_store |= other.mutates_memory_store;
        self.writes_ndkv |= other.writes_ndkv;
        self.starts_daemon |= other.starts_daemon;
        self.stops_daemon |= other.stops_daemon;
        self.touches_remote |= other.touches_remote;
        self.downloads_model |= other.downloads_model;
        self.warms_model_cache |= other.warms_model_cache;
        self.starts_forge |= other.starts_forge;
        self.starts_web_lab |= other.starts_web_lab;
        self.sends_prompt |= other.sends_prompt;
        self.starts_stream |= other.starts_stream;
        self.replays_prompt |= other.replays_prompt;
        self.calls_model |= other.calls_model;
    }

    fn safe(&self) -> bool {
        !self.applies_code
            && !self.edits_files
            && !self.mutates_ledger
            && !self.mutates_memory_store
            && !self.writes_ndkv
            && !self.starts_daemon
            && !self.stops_daemon
            && !self.touches_remote
            && !self.downloads_model
            && !self.warms_model_cache
            && !self.starts_forge
            && !self.starts_web_lab
            && !self.sends_prompt
            && !self.starts_stream
            && !self.replays_prompt
            && !self.calls_model
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"applies_code\":{},\"edits_files\":{},\"mutates_ledger\":{},\"mutates_memory_store\":{},\"writes_ndkv\":{},\"starts_daemon\":{},\"stops_daemon\":{},\"touches_remote\":{},\"downloads_model\":{},\"warms_model_cache\":{},\"starts_forge\":{},\"starts_web_lab\":{},\"sends_prompt\":{},\"starts_stream\":{},\"replays_prompt\":{},\"calls_model\":{}}}",
            bool_value_text(self.applies_code),
            bool_value_text(self.edits_files),
            bool_value_text(self.mutates_ledger),
            bool_value_text(self.mutates_memory_store),
            bool_value_text(self.writes_ndkv),
            bool_value_text(self.starts_daemon),
            bool_value_text(self.stops_daemon),
            bool_value_text(self.touches_remote),
            bool_value_text(self.downloads_model),
            bool_value_text(self.warms_model_cache),
            bool_value_text(self.starts_forge),
            bool_value_text(self.starts_web_lab),
            bool_value_text(self.sends_prompt),
            bool_value_text(self.starts_stream),
            bool_value_text(self.replays_prompt),
            bool_value_text(self.calls_model)
        )
    }
}

fn helper_stage_repair_report(report_json: &str) -> Option<&str> {
    json_top_level_object_field(report_json, "helper_stage_repair_status_report_v1")
        .or_else(|| json_top_level_object_field(report_json, "helper_stage_repair_panel_v1"))
        .or_else(|| {
            (json_top_level_string_field(report_json, "schema").as_deref()
                == Some("helper_stage_repair_status_report_v1"))
            .then_some(report_json)
        })
}

fn bool_from(object: Option<&str>, field: &str) -> bool {
    object
        .and_then(|value| json_bool_field(value, field))
        .unwrap_or(false)
}

fn number_or_count(object: &str, field: &str, fallback: usize) -> String {
    json_number_field(object, field).unwrap_or_else(|| fallback.to_string())
}

fn number_or_count_from_fields(object: &str, fields: &[&str], fallback: usize) -> String {
    fields
        .iter()
        .find_map(|field| json_number_field(object, field))
        .unwrap_or_else(|| fallback.to_string())
}

fn count_is_nonzero(value: &str) -> bool {
    value
        .parse::<usize>()
        .map(|count| count > 0)
        .unwrap_or(false)
}

fn is_missing_helper_role_status(status: &str) -> bool {
    matches!(
        status,
        "missing_helper_role"
            | "missing_required_helper_role"
            | "missing_helper_role_repair_required"
    )
}

fn compact_string_field(object: &str, field: &str) -> String {
    json_string_field(object, field)
        .map(|value| compact_line(&value, 160))
        .unwrap_or_else(|| "unknown".to_owned())
}

fn compact_array_field(object: &str, field: &str) -> Vec<String> {
    json_string_array_field(object, field)
        .unwrap_or_default()
        .into_iter()
        .map(|value| compact_line(&value, 120))
        .collect()
}

fn push_unique(target: &mut Vec<String>, value: String) {
    if !target.iter().any(|existing| existing == &value) {
        target.push(value);
    }
}

fn push_unique_all(target: &mut Vec<String>, values: &[String]) {
    for value in values {
        push_unique(target, value.clone());
    }
}

fn list_value(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_owned()
    } else {
        values.join(",")
    }
}

fn proposals_json(proposals: &[HelperStageRepairProposal]) -> String {
    let mut out = String::from("[");
    for (index, proposal) in proposals.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&proposal.to_json());
    }
    out.push(']');
    out
}

fn string_array_json(values: &[String]) -> String {
    let mut out = String::from("[");
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&json_string_literal(value));
    }
    out.push(']');
    out
}

fn optional_string_json(value: Option<&str>) -> String {
    value
        .map(json_string_literal)
        .unwrap_or_else(|| "null".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_stage_repair_panel_projects_repair_status_without_raw_payloads() {
        let report = include_str!("../../fixtures/r28-helper-stage-repair-status.example.json");

        let lines = helper_stage_repair_panel_lines(report).join("\n");
        let json = helper_stage_repair_panel_json(Some(report));

        assert!(lines.contains("helper_stage_repair_panel read_only=true starts_process=false sends_prompt=false status_loaded=true report_only=true safe=true source=latest_ledger_helper_stage_contract_projection latest_round=328 repair_required=true total_roles=4 incomplete_roles=2 missing_helper_role_repair_required=true missing_helper_role_repair_proposals=1 missing_helper_roles=router proposals=3 roles=router,review,test-gate missing_fields=verification,failure_kind placeholder_fields=validation_command starts_daemon=false starts_forge=false starts_web_lab=false calls_model=false starts_stream=false replays_prompt=false writes_ndkv=false mutates_memory_store=false auto_apply=false"));
        assert!(lines.contains("helper_stage_repair_proposal id=helper-stage-repair-r328-router role=router status=missing_helper_role missing_helper_role_repair_required=true missing_fields=none placeholder_fields=none validation_safety=safe candidate_only=true auto_apply=false"));
        assert!(lines.contains("helper_stage_repair_proposal id=helper-stage-repair-r328-review role=review status=missing_required_fields missing_helper_role_repair_required=false missing_fields=verification placeholder_fields=none validation_safety=safe candidate_only=true auto_apply=false"));
        assert!(json.contains("\"status_loaded\":true"));
        assert!(json.contains("\"repair_required\":true"));
        assert!(json.contains("\"missing_helper_role_repair_required\":true"));
        assert!(json.contains("\"missing_helper_role_repair_proposal_count\":1"));
        assert!(json.contains("\"missing_helper_roles\":[\"router\"]"));
        assert!(json.contains("\"proposal_count\":3"));
        assert!(json.contains("\"roles\":[\"router\",\"review\",\"test-gate\"]"));
        assert!(json.contains("\"starts_web_lab\":false"));
        assert!(json.contains("\"calls_model\":false"));
        assert!(json.contains("\"auto_apply\":false"));
        assert!(!json.contains("RAW_OLD_WINDOW_PAYLOAD"));
        assert!(!json.contains("helper says replay prompt"));
        assert!(!json.contains("/v1/chat-stream"));
    }

    #[test]
    fn helper_stage_repair_panel_defaults_to_safe_empty_json_when_absent() {
        let json = helper_stage_repair_panel_json(None);

        assert!(helper_stage_repair_panel_lines("{}").is_empty());
        assert!(json.contains("\"read_only\":true"));
        assert!(json.contains("\"status_loaded\":false"));
        assert!(json.contains("\"repair_required\":false"));
        assert!(json.contains("\"safe\":true"));
        assert!(json.contains("\"missing_helper_role_repair_required\":false"));
        assert!(json.contains("\"missing_helper_role_repair_proposal_count\":0"));
        assert!(json.contains("\"proposal_count\":0"));
        assert!(json.contains("\"starts_web_lab\":false"));
        assert!(json.contains("\"calls_model\":false"));
    }
}
