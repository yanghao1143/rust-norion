use super::status_json::{
    bool_value_text, compact_line, json_bool_field, json_object_array_field, json_object_field,
    json_string_array_field, json_string_field, json_string_literal, json_top_level_object_field,
    scalar_value,
};

pub(super) fn self_improve_proposal_panel_lines(report_json: &str) -> Vec<String> {
    let Some(panel) = SelfImproveProposalPanel::from_report_json(Some(report_json)) else {
        return Vec::new();
    };

    vec![
        format!(
            "self_improve_proposal_panel read_only={} starts_process={} sends_prompt={} status_loaded={} report_only={} safe={} source={} candidate={} validated={} admitted={} quarantined={} promoted={} repair_required={} starts_daemon={} stops_daemon={} starts_stream={} replays_prompt={} touches_remote={} downloads_model={} warms_model_cache={}",
            bool_value_text(panel.read_only),
            bool_value_text(panel.starts_process),
            bool_value_text(panel.sends_prompt),
            bool_value_text(panel.status_loaded),
            bool_value_text(panel.report_only),
            bool_value_text(panel.safe),
            panel.source.as_deref().unwrap_or("unknown"),
            panel.candidate.count,
            panel.validated.count,
            panel.admitted.count,
            panel.quarantined.count,
            panel.promoted.count,
            panel.repair_required.count,
            bool_value_text(panel.side_effects.starts_daemon),
            bool_value_text(panel.side_effects.stops_daemon),
            bool_value_text(panel.side_effects.starts_stream),
            bool_value_text(panel.side_effects.replays_prompt),
            bool_value_text(panel.side_effects.touches_remote),
            bool_value_text(panel.side_effects.downloads_model),
            bool_value_text(panel.side_effects.warms_model_cache)
        ),
        panel.candidate.line("candidate"),
        panel.validated.line("validated"),
        panel.admitted.line("admitted"),
        panel.quarantined.line("quarantined"),
        panel.promoted.line("promoted"),
        panel.repair_required.line("repair_required"),
        panel.prompt_guidance.line(),
        panel.action_plan.line(),
        panel.action_assignment.line(),
    ]
}

pub(super) fn self_improve_proposal_panel_json(report_json: Option<&str>) -> String {
    SelfImproveProposalPanel::from_report_json(report_json)
        .unwrap_or_default()
        .to_json()
}

struct SelfImproveProposalPanel {
    read_only: bool,
    starts_process: bool,
    sends_prompt: bool,
    status_loaded: bool,
    report_only: bool,
    source: Option<String>,
    candidate: ProposalStage,
    validated: ProposalStage,
    admitted: ProposalStage,
    quarantined: ProposalStage,
    promoted: ProposalStage,
    repair_required: ProposalStage,
    prompt_guidance: ProposalPromptGuidance,
    action_plan: ProposalActionPlan,
    action_assignment: ProposalActionAssignment,
    side_effects: ProposalSideEffects,
    safe: bool,
}

impl Default for SelfImproveProposalPanel {
    fn default() -> Self {
        Self {
            read_only: true,
            starts_process: false,
            sends_prompt: false,
            status_loaded: false,
            report_only: true,
            source: None,
            candidate: ProposalStage::default(),
            validated: ProposalStage::default(),
            admitted: ProposalStage::default(),
            quarantined: ProposalStage::default(),
            promoted: ProposalStage::default(),
            repair_required: ProposalStage::default(),
            prompt_guidance: ProposalPromptGuidance::default(),
            action_plan: ProposalActionPlan::default(),
            action_assignment: ProposalActionAssignment::default(),
            side_effects: ProposalSideEffects::default(),
            safe: true,
        }
    }
}

impl SelfImproveProposalPanel {
    fn from_report_json(report_json: Option<&str>) -> Option<Self> {
        let full_report = report_json?;
        let report = json_top_level_object_field(full_report, "self_improve_proposal_artifact_v1")
            .or_else(|| {
                json_top_level_object_field(full_report, "self_improve_proposal_panel_v1")
            })?;
        let acceptance_summary =
            json_top_level_object_field(full_report, "self_improve_proposal_acceptance_summary_v1");
        let evidence = json_object_field(report, "evidence_map");
        let lifecycle = json_object_field(report, "lifecycle");
        let side_effects =
            ProposalSideEffects::from_json(json_object_field(report, "side_effects"));
        let read_only = json_bool_field(report, "read_only").unwrap_or(true);
        let starts_process = json_bool_field(report, "starts_process").unwrap_or(false);
        let sends_prompt =
            json_bool_field(report, "sends_prompt").unwrap_or(side_effects.sends_prompt);
        let report_only = json_bool_field(report, "report_only").unwrap_or(true);
        let prompt_guidance = ProposalPromptGuidance::from_sources(report, acceptance_summary);
        let action_plan =
            ProposalActionPlan::from_sources(report, acceptance_summary, &prompt_guidance);
        let action_assignment = ProposalActionAssignment::from_sources(report, acceptance_summary);
        let safe = read_only
            && !starts_process
            && !sends_prompt
            && report_only
            && side_effects.safe()
            && action_plan.safe
            && action_assignment.safe;

        Some(Self {
            read_only,
            starts_process,
            sends_prompt,
            status_loaded: true,
            report_only,
            source: json_string_field(report, "source").map(|value| compact_line(&value, 160)),
            candidate: ProposalStage::from_sources("candidate", report, lifecycle, evidence),
            validated: ProposalStage::from_sources("validated", report, lifecycle, evidence),
            admitted: ProposalStage::from_sources("admitted", report, lifecycle, evidence),
            quarantined: ProposalStage::from_sources("quarantined", report, lifecycle, evidence),
            promoted: ProposalStage::from_sources("promoted", report, lifecycle, evidence),
            repair_required: ProposalStage::from_sources(
                "repair_required",
                report,
                lifecycle,
                evidence,
            ),
            prompt_guidance,
            action_plan,
            action_assignment,
            side_effects,
            safe,
        })
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"read_only\":{},\"starts_process\":{},\"sends_prompt\":{},\"status_loaded\":{},\"report_only\":{},\"safe\":{},\"source\":{},\"candidate_count\":{},\"validated_count\":{},\"admitted_count\":{},\"quarantined_count\":{},\"promoted_count\":{},\"repair_required_count\":{},\"candidate\":{},\"validated\":{},\"admitted\":{},\"quarantined\":{},\"promoted\":{},\"repair_required\":{},\"prompt_guidance\":{},\"action_plan\":{},\"action_assignment\":{},\"side_effects\":{}}}",
            bool_value_text(self.read_only),
            bool_value_text(self.starts_process),
            bool_value_text(self.sends_prompt),
            bool_value_text(self.status_loaded),
            bool_value_text(self.report_only),
            bool_value_text(self.safe),
            optional_string_json(self.source.as_deref()),
            self.candidate.count,
            self.validated.count,
            self.admitted.count,
            self.quarantined.count,
            self.promoted.count,
            self.repair_required.count,
            self.candidate.to_json(),
            self.validated.to_json(),
            self.admitted.to_json(),
            self.quarantined.to_json(),
            self.promoted.to_json(),
            self.repair_required.to_json(),
            self.prompt_guidance.to_json(),
            self.action_plan.to_json(),
            self.action_assignment.to_json(),
            self.side_effects.to_json()
        )
    }
}

struct ProposalStage {
    count: String,
    ids: Vec<String>,
    reason_codes: Vec<String>,
}

impl Default for ProposalStage {
    fn default() -> Self {
        Self {
            count: "0".to_owned(),
            ids: Vec::new(),
            reason_codes: Vec::new(),
        }
    }
}

impl ProposalStage {
    fn from_sources(
        stage: &str,
        report: &str,
        lifecycle: Option<&str>,
        evidence: Option<&str>,
    ) -> Self {
        let stage_object = json_object_field(report, stage)
            .or_else(|| lifecycle.and_then(|value| json_object_field(value, stage)))
            .or_else(|| evidence.and_then(|value| json_object_field(value, stage)));
        let ids = stage_ids(stage, stage_object, report, lifecycle, evidence);
        let reason_codes = stage_reason_codes(stage, stage_object, report, lifecycle, evidence);
        let count = stage_count(stage, stage_object, report, lifecycle, evidence, ids.len());

        Self {
            count,
            ids,
            reason_codes,
        }
    }

    fn line(&self, stage: &str) -> String {
        format!(
            "self_improve_proposal_{stage} count={} ids={} reason_codes={}",
            self.count,
            list_value(&self.ids),
            list_value(&self.reason_codes)
        )
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"read_only\":true,\"starts_process\":false,\"sends_prompt\":false,\"count\":{},\"ids\":{},\"reason_codes\":{}}}",
            self.count,
            string_array_json(&self.ids),
            string_array_json(&self.reason_codes)
        )
    }
}

#[derive(Default)]
struct ProposalPromptGuidance {
    status_loaded: bool,
    convert_advisory_to_business_evidence: bool,
    repair_unvalidated_or_unaccepted: bool,
    requires_validation_and_memory_admission: bool,
    evidence_ids: Vec<String>,
}

impl ProposalPromptGuidance {
    fn from_sources(report: &str, acceptance_summary: Option<&str>) -> Self {
        let report_guidance = json_object_field(report, "prompt_guidance");
        let summary_guidance =
            acceptance_summary.and_then(|value| json_object_field(value, "prompt_guidance"));
        let convert_advisory_to_business_evidence = bool_from_any(
            [
                report_guidance,
                summary_guidance,
                Some(report),
                acceptance_summary,
            ],
            [
                "convert_advisory_to_business_evidence",
                "should_convert_advisory_to_evidence_backed_business_improvement",
            ],
        )
        .unwrap_or(false);
        let repair_unvalidated_or_unaccepted = bool_from_any(
            [
                report_guidance,
                summary_guidance,
                Some(report),
                acceptance_summary,
            ],
            [
                "repair_unvalidated_or_unaccepted",
                "should_repair_unvalidated_or_unaccepted_proposals",
            ],
        )
        .unwrap_or(false);
        let requires_validation_and_memory_admission = bool_from_any(
            [
                report_guidance,
                summary_guidance,
                Some(report),
                acceptance_summary,
            ],
            [
                "requires_validation_and_memory_admission",
                "requires_checked_passed_validation_and_accepted_memory_admission",
            ],
        )
        .unwrap_or(false);
        let evidence_ids = array_from(report_guidance, "evidence_ids")
            .or_else(|| array_from(summary_guidance, "evidence_ids"))
            .unwrap_or_default();
        let status_loaded = report_guidance.is_some()
            || summary_guidance.is_some()
            || bool_from_any(
                [Some(report), acceptance_summary],
                [
                    "convert_advisory_to_business_evidence",
                    "should_convert_advisory_to_evidence_backed_business_improvement",
                    "repair_unvalidated_or_unaccepted",
                    "should_repair_unvalidated_or_unaccepted_proposals",
                    "requires_validation_and_memory_admission",
                    "requires_checked_passed_validation_and_accepted_memory_admission",
                ],
            )
            .is_some();

        Self {
            status_loaded,
            convert_advisory_to_business_evidence,
            repair_unvalidated_or_unaccepted,
            requires_validation_and_memory_admission,
            evidence_ids,
        }
    }

    fn line(&self) -> String {
        format!(
            "self_improve_proposal_guidance read_only=true starts_process=false sends_prompt=false status_loaded={} report_only=true safe=true convert_advisory_to_business_evidence={} repair_unvalidated_or_unaccepted={} requires_validation_and_memory_admission={} evidence_ids={}",
            bool_value_text(self.status_loaded),
            bool_value_text(self.convert_advisory_to_business_evidence),
            bool_value_text(self.repair_unvalidated_or_unaccepted),
            bool_value_text(self.requires_validation_and_memory_admission),
            list_value(&self.evidence_ids)
        )
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"read_only\":true,\"starts_process\":false,\"sends_prompt\":false,\"status_loaded\":{},\"report_only\":true,\"safe\":true,\"convert_advisory_to_business_evidence\":{},\"repair_unvalidated_or_unaccepted\":{},\"requires_validation_and_memory_admission\":{},\"evidence_ids\":{}}}",
            bool_value_text(self.status_loaded),
            bool_value_text(self.convert_advisory_to_business_evidence),
            bool_value_text(self.repair_unvalidated_or_unaccepted),
            bool_value_text(self.requires_validation_and_memory_admission),
            string_array_json(&self.evidence_ids)
        )
    }
}

struct ProposalActionPlan {
    status_loaded: bool,
    safe: bool,
    action_required: bool,
    primary_action: String,
    actions: Vec<String>,
    requires_validation_and_memory_admission: bool,
    auto_apply: bool,
}

impl Default for ProposalActionPlan {
    fn default() -> Self {
        Self {
            status_loaded: false,
            safe: true,
            action_required: false,
            primary_action: "none".to_owned(),
            actions: Vec::new(),
            requires_validation_and_memory_admission: false,
            auto_apply: false,
        }
    }
}

impl ProposalActionPlan {
    fn from_sources(
        report: &str,
        acceptance_summary: Option<&str>,
        guidance: &ProposalPromptGuidance,
    ) -> Self {
        let report_plan = json_object_field(report, "action_plan");
        let summary_plan =
            acceptance_summary.and_then(|value| json_object_field(value, "action_plan"));
        let source_plan = summary_plan.or(report_plan);
        let mut actions = array_from(source_plan, "actions")
            .or_else(|| array_from(acceptance_summary, "actions"))
            .or_else(|| array_from(Some(report), "actions"))
            .unwrap_or_default();
        let mut status_loaded = source_plan.is_some()
            || bool_from_any([Some(report), acceptance_summary], ["action_required"]).is_some()
            || !actions.is_empty();

        if actions.is_empty() && guidance.status_loaded {
            actions = derived_actions_from_guidance(guidance);
            status_loaded = true;
        }

        let primary_action = source_plan
            .and_then(|value| json_string_field(value, "primary_action"))
            .or_else(|| {
                acceptance_summary.and_then(|value| json_string_field(value, "primary_action"))
            })
            .or_else(|| json_string_field(report, "primary_action"))
            .or_else(|| actions.first().cloned())
            .unwrap_or_else(|| "none".to_owned());
        let action_required = bool_from_any(
            [source_plan, acceptance_summary, Some(report)],
            ["action_required"],
        )
        .unwrap_or(!actions.is_empty());
        let requires_validation_and_memory_admission = bool_from_any(
            [source_plan, acceptance_summary, Some(report)],
            [
                "requires_validation_and_memory_admission",
                "requires_checked_passed_validation_and_accepted_memory_admission",
            ],
        )
        .unwrap_or(guidance.requires_validation_and_memory_admission);
        let read_only = source_plan
            .and_then(|value| json_bool_field(value, "read_only"))
            .unwrap_or(true);
        let starts_process = source_plan
            .and_then(|value| json_bool_field(value, "starts_process"))
            .unwrap_or(false);
        let sends_prompt = source_plan
            .and_then(|value| json_bool_field(value, "sends_prompt"))
            .unwrap_or(false);
        let report_only = source_plan
            .and_then(|value| json_bool_field(value, "report_only"))
            .unwrap_or(true);
        let auto_apply = source_plan
            .and_then(|value| json_bool_field(value, "auto_apply"))
            .unwrap_or(false);
        let explicit_safe = source_plan
            .and_then(|value| json_bool_field(value, "safe"))
            .unwrap_or(true);

        Self {
            status_loaded,
            safe: read_only
                && !starts_process
                && !sends_prompt
                && report_only
                && !auto_apply
                && explicit_safe,
            action_required,
            primary_action,
            actions,
            requires_validation_and_memory_admission,
            auto_apply,
        }
    }

    fn line(&self) -> String {
        format!(
            "self_improve_proposal_action_plan read_only=true starts_process=false sends_prompt=false status_loaded={} report_only=true safe={} action_required={} primary_action={} actions={} requires_validation_and_memory_admission={} auto_apply={}",
            bool_value_text(self.status_loaded),
            bool_value_text(self.safe),
            bool_value_text(self.action_required),
            self.primary_action,
            list_value(&self.actions),
            bool_value_text(self.requires_validation_and_memory_admission),
            bool_value_text(self.auto_apply)
        )
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"read_only\":true,\"starts_process\":false,\"sends_prompt\":false,\"status_loaded\":{},\"report_only\":true,\"safe\":{},\"action_required\":{},\"primary_action\":{},\"actions\":{},\"requires_validation_and_memory_admission\":{},\"auto_apply\":{}}}",
            bool_value_text(self.status_loaded),
            bool_value_text(self.safe),
            bool_value_text(self.action_required),
            json_string_literal(&self.primary_action),
            string_array_json(&self.actions),
            bool_value_text(self.requires_validation_and_memory_admission),
            bool_value_text(self.auto_apply)
        )
    }
}

struct ProposalActionAssignment {
    status_loaded: bool,
    safe: bool,
    action_required: bool,
    primary_action: String,
    actions: Vec<String>,
    target_count: String,
    first_target: String,
    first_source_round: String,
    first_evidence_ids: Vec<String>,
    first_memory_admission_decision: String,
    first_validation_checked: bool,
    first_validation_passed: bool,
    first_memory_admission_accepted: bool,
    first_evidence_backed_business_improvement: bool,
    first_advisory_only: bool,
    first_require_repair: bool,
    first_missing_requirements: Vec<String>,
    requires_validation_and_memory_admission: bool,
    auto_apply: bool,
}

impl Default for ProposalActionAssignment {
    fn default() -> Self {
        Self {
            status_loaded: false,
            safe: true,
            action_required: false,
            primary_action: "none".to_owned(),
            actions: Vec::new(),
            target_count: "0".to_owned(),
            first_target: "none".to_owned(),
            first_source_round: "null".to_owned(),
            first_evidence_ids: Vec::new(),
            first_memory_admission_decision: "unknown".to_owned(),
            first_validation_checked: false,
            first_validation_passed: false,
            first_memory_admission_accepted: false,
            first_evidence_backed_business_improvement: false,
            first_advisory_only: false,
            first_require_repair: false,
            first_missing_requirements: Vec::new(),
            requires_validation_and_memory_admission: false,
            auto_apply: false,
        }
    }
}

impl ProposalActionAssignment {
    fn from_sources(report: &str, acceptance_summary: Option<&str>) -> Self {
        let report_assignment = json_object_field(report, "action_assignment");
        let summary_assignment =
            acceptance_summary.and_then(|value| json_object_field(value, "action_assignment"));
        let Some(source_assignment) = summary_assignment.or(report_assignment) else {
            return Self::default();
        };

        let actions = array_from(Some(source_assignment), "actions").unwrap_or_default();
        let target_count = scalar_from(source_assignment, "target_count").unwrap_or_else(|| {
            json_object_array_field(source_assignment, "targets")
                .map(|targets| targets.len().to_string())
                .unwrap_or_else(|| "0".to_owned())
        });
        let first_target_object = json_object_array_field(source_assignment, "targets")
            .and_then(|targets| targets.first().copied());
        let first_target = first_target_object
            .and_then(|target| json_string_field(target, "proposal_id"))
            .unwrap_or_else(|| "none".to_owned());
        let first_source_round = first_target_object
            .and_then(|target| scalar_from(target, "source_round"))
            .unwrap_or_else(|| "null".to_owned());
        let first_evidence_ids = first_target_object
            .and_then(|target| array_from(Some(target), "evidence_ids"))
            .unwrap_or_default();
        let first_memory_admission_decision = first_target_object
            .and_then(|target| json_string_field(target, "current_memory_admission_decision"))
            .unwrap_or_else(|| "unknown".to_owned());
        let first_validation_checked = first_target_object
            .and_then(|target| json_bool_field(target, "validation_checked"))
            .unwrap_or(false);
        let first_validation_passed = first_target_object
            .and_then(|target| json_bool_field(target, "validation_passed"))
            .unwrap_or(false);
        let first_memory_admission_accepted = first_target_object
            .and_then(|target| json_bool_field(target, "memory_admission_accepted"))
            .unwrap_or(false);
        let first_evidence_backed_business_improvement = first_target_object
            .and_then(|target| json_bool_field(target, "evidence_backed_business_improvement"))
            .unwrap_or(false);
        let first_advisory_only = first_target_object
            .and_then(|target| json_bool_field(target, "advisory_only"))
            .unwrap_or(false);
        let first_require_repair = first_target_object
            .and_then(|target| json_bool_field(target, "require_repair"))
            .unwrap_or(false);
        let first_missing_requirements = first_target_object
            .and_then(|target| array_from(Some(target), "missing_requirements"))
            .unwrap_or_default();
        let action_required =
            json_bool_field(source_assignment, "action_required").unwrap_or(target_count != "0");
        let primary_action = json_string_field(source_assignment, "primary_action")
            .or_else(|| actions.first().cloned())
            .unwrap_or_else(|| "none".to_owned());
        let requires_validation_and_memory_admission = bool_from_any(
            [Some(source_assignment)],
            [
                "requires_validation_and_memory_admission",
                "requires_checked_passed_validation_and_accepted_memory_admission",
            ],
        )
        .unwrap_or(false);
        let read_only = json_bool_field(source_assignment, "read_only").unwrap_or(true);
        let starts_process = json_bool_field(source_assignment, "starts_process").unwrap_or(false);
        let sends_prompt = json_bool_field(source_assignment, "sends_prompt").unwrap_or(false);
        let report_only = json_bool_field(source_assignment, "report_only").unwrap_or(true);
        let auto_apply = json_bool_field(source_assignment, "auto_apply").unwrap_or(false);
        let explicit_safe = json_bool_field(source_assignment, "safe").unwrap_or(true);
        let side_effects =
            ProposalSideEffects::from_json(json_object_field(source_assignment, "side_effects"));

        Self {
            status_loaded: true,
            safe: read_only
                && !starts_process
                && !sends_prompt
                && report_only
                && !auto_apply
                && explicit_safe
                && side_effects.safe(),
            action_required,
            primary_action,
            actions,
            target_count,
            first_target,
            first_source_round,
            first_evidence_ids,
            first_memory_admission_decision,
            first_validation_checked,
            first_validation_passed,
            first_memory_admission_accepted,
            first_evidence_backed_business_improvement,
            first_advisory_only,
            first_require_repair,
            first_missing_requirements,
            requires_validation_and_memory_admission,
            auto_apply,
        }
    }

    fn line(&self) -> String {
        format!(
            "self_improve_proposal_action_assignment read_only=true starts_process=false sends_prompt=false status_loaded={} report_only=true safe={} action_required={} primary_action={} actions={} target_count={} first_target={} first_source_round={} first_evidence_ids={} first_memory_admission_decision={} first_validation_checked={} first_validation_passed={} first_memory_admission_accepted={} first_evidence_backed_business_improvement={} first_advisory_only={} first_require_repair={} first_missing_requirements={} requires_validation_and_memory_admission={} auto_apply={}",
            bool_value_text(self.status_loaded),
            bool_value_text(self.safe),
            bool_value_text(self.action_required),
            self.primary_action,
            list_value(&self.actions),
            self.target_count,
            self.first_target,
            self.first_source_round,
            list_value(&self.first_evidence_ids),
            self.first_memory_admission_decision,
            bool_value_text(self.first_validation_checked),
            bool_value_text(self.first_validation_passed),
            bool_value_text(self.first_memory_admission_accepted),
            bool_value_text(self.first_evidence_backed_business_improvement),
            bool_value_text(self.first_advisory_only),
            bool_value_text(self.first_require_repair),
            list_value(&self.first_missing_requirements),
            bool_value_text(self.requires_validation_and_memory_admission),
            bool_value_text(self.auto_apply)
        )
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"read_only\":true,\"starts_process\":false,\"sends_prompt\":false,\"status_loaded\":{},\"report_only\":true,\"safe\":{},\"action_required\":{},\"primary_action\":{},\"actions\":{},\"target_count\":{},\"first_target\":{},\"first_source_round\":{},\"first_evidence_ids\":{},\"first_memory_admission_decision\":{},\"first_validation_checked\":{},\"first_validation_passed\":{},\"first_memory_admission_accepted\":{},\"first_evidence_backed_business_improvement\":{},\"first_advisory_only\":{},\"first_require_repair\":{},\"first_missing_requirements\":{},\"requires_validation_and_memory_admission\":{},\"auto_apply\":{}}}",
            bool_value_text(self.status_loaded),
            bool_value_text(self.safe),
            bool_value_text(self.action_required),
            json_string_literal(&self.primary_action),
            string_array_json(&self.actions),
            self.target_count,
            json_string_literal(&self.first_target),
            self.first_source_round,
            string_array_json(&self.first_evidence_ids),
            json_string_literal(&self.first_memory_admission_decision),
            bool_value_text(self.first_validation_checked),
            bool_value_text(self.first_validation_passed),
            bool_value_text(self.first_memory_admission_accepted),
            bool_value_text(self.first_evidence_backed_business_improvement),
            bool_value_text(self.first_advisory_only),
            bool_value_text(self.first_require_repair),
            string_array_json(&self.first_missing_requirements),
            bool_value_text(self.requires_validation_and_memory_admission),
            bool_value_text(self.auto_apply)
        )
    }
}

fn derived_actions_from_guidance(guidance: &ProposalPromptGuidance) -> Vec<String> {
    let mut actions = Vec::new();
    if guidance.convert_advisory_to_business_evidence {
        actions.push("convert_advisory_to_evidence_backed_business_improvement".to_owned());
    }
    if guidance.repair_unvalidated_or_unaccepted {
        actions.push("repair_unvalidated_or_unaccepted_proposals".to_owned());
    }
    if guidance.requires_validation_and_memory_admission {
        actions.push("require_checked_passed_validation_and_accepted_memory_admission".to_owned());
    }
    actions
}

#[derive(Default)]
struct ProposalSideEffects {
    starts_daemon: bool,
    stops_daemon: bool,
    starts_process: bool,
    touches_remote: bool,
    downloads_model: bool,
    warms_model_cache: bool,
    sends_prompt: bool,
    starts_stream: bool,
    replays_prompt: bool,
    starts_thread: bool,
    sends_message: bool,
    mutates_memory_store: bool,
    writes_ndkv: bool,
    promotes_candidate: bool,
    repairs_artifact: bool,
}

impl ProposalSideEffects {
    fn from_json(side_effects: Option<&str>) -> Self {
        Self {
            starts_daemon: bool_from(side_effects, "starts_daemon").unwrap_or(false),
            stops_daemon: bool_from(side_effects, "stops_daemon").unwrap_or(false),
            starts_process: bool_from(side_effects, "starts_process").unwrap_or(false),
            touches_remote: bool_from(side_effects, "touches_remote").unwrap_or(false),
            downloads_model: bool_from(side_effects, "downloads_model").unwrap_or(false),
            warms_model_cache: bool_from(side_effects, "warms_model_cache").unwrap_or(false),
            sends_prompt: bool_from(side_effects, "sends_prompt").unwrap_or(false),
            starts_stream: bool_from(side_effects, "starts_stream").unwrap_or(false),
            replays_prompt: bool_from(side_effects, "replays_prompt").unwrap_or(false),
            starts_thread: bool_from(side_effects, "starts_thread").unwrap_or(false),
            sends_message: bool_from(side_effects, "sends_message").unwrap_or(false),
            mutates_memory_store: bool_from(side_effects, "mutates_memory_store").unwrap_or(false),
            writes_ndkv: bool_from(side_effects, "writes_ndkv").unwrap_or(false),
            promotes_candidate: bool_from(side_effects, "promotes_candidate").unwrap_or(false),
            repairs_artifact: bool_from(side_effects, "repairs_artifact").unwrap_or(false),
        }
    }

    fn safe(&self) -> bool {
        !self.starts_daemon
            && !self.stops_daemon
            && !self.starts_process
            && !self.touches_remote
            && !self.downloads_model
            && !self.warms_model_cache
            && !self.sends_prompt
            && !self.starts_stream
            && !self.replays_prompt
            && !self.starts_thread
            && !self.sends_message
            && !self.mutates_memory_store
            && !self.writes_ndkv
            && !self.promotes_candidate
            && !self.repairs_artifact
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"starts_daemon\":{},\"stops_daemon\":{},\"starts_process\":{},\"touches_remote\":{},\"downloads_model\":{},\"warms_model_cache\":{},\"sends_prompt\":{},\"starts_stream\":{},\"replays_prompt\":{},\"starts_thread\":{},\"sends_message\":{},\"mutates_memory_store\":{},\"writes_ndkv\":{},\"promotes_candidate\":{},\"repairs_artifact\":{}}}",
            bool_value_text(self.starts_daemon),
            bool_value_text(self.stops_daemon),
            bool_value_text(self.starts_process),
            bool_value_text(self.touches_remote),
            bool_value_text(self.downloads_model),
            bool_value_text(self.warms_model_cache),
            bool_value_text(self.sends_prompt),
            bool_value_text(self.starts_stream),
            bool_value_text(self.replays_prompt),
            bool_value_text(self.starts_thread),
            bool_value_text(self.sends_message),
            bool_value_text(self.mutates_memory_store),
            bool_value_text(self.writes_ndkv),
            bool_value_text(self.promotes_candidate),
            bool_value_text(self.repairs_artifact)
        )
    }
}

fn stage_count(
    stage: &str,
    stage_object: Option<&str>,
    report: &str,
    lifecycle: Option<&str>,
    evidence: Option<&str>,
    id_count: usize,
) -> String {
    stage_object
        .and_then(|value| scalar_from(value, "count"))
        .or_else(|| scalar_from(report, &format!("{stage}_count")))
        .or_else(|| lifecycle.and_then(|value| scalar_from(value, &format!("{stage}_count"))))
        .or_else(|| evidence.and_then(|value| scalar_from(value, &format!("{stage}_count"))))
        .unwrap_or_else(|| id_count.to_string())
}

fn stage_ids(
    stage: &str,
    stage_object: Option<&str>,
    report: &str,
    lifecycle: Option<&str>,
    evidence: Option<&str>,
) -> Vec<String> {
    array_from(stage_object, "ids")
        .or_else(|| array_from(Some(report), &format!("{stage}_ids")))
        .or_else(|| array_from(lifecycle, &format!("{stage}_ids")))
        .or_else(|| array_from(evidence, &format!("{stage}_ids")))
        .unwrap_or_default()
}

fn stage_reason_codes(
    stage: &str,
    stage_object: Option<&str>,
    report: &str,
    lifecycle: Option<&str>,
    evidence: Option<&str>,
) -> Vec<String> {
    array_from(stage_object, "reason_codes")
        .or_else(|| array_from(Some(report), &format!("{stage}_reason_codes")))
        .or_else(|| array_from(lifecycle, &format!("{stage}_reason_codes")))
        .or_else(|| array_from(evidence, &format!("{stage}_reason_codes")))
        .unwrap_or_default()
}

fn bool_from(object: Option<&str>, field: &str) -> Option<bool> {
    object.and_then(|value| json_bool_field(value, field))
}

fn bool_from_any<const N: usize, const M: usize>(
    objects: [Option<&str>; N],
    fields: [&str; M],
) -> Option<bool> {
    for object in objects.into_iter().flatten() {
        for field in fields {
            if let Some(value) = json_bool_field(object, field) {
                return Some(value);
            }
        }
    }
    None
}

fn scalar_from(object: &str, field: &str) -> Option<String> {
    let value = scalar_value(object, field);
    (value != "unknown").then_some(value)
}

fn array_from(object: Option<&str>, field: &str) -> Option<Vec<String>> {
    object.and_then(|value| json_string_array_field(value, field))
}

fn list_value(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_owned()
    } else {
        values.join(",")
    }
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
    fn proposal_panel_projects_lifecycle_without_raw_payloads() {
        let report = include_str!("../../fixtures/r26-self-improve-proposal-artifact.example.json");

        let lines = self_improve_proposal_panel_lines(report).join("\n");
        let json = self_improve_proposal_panel_json(Some(report));

        assert!(lines.contains("self_improve_proposal_panel read_only=true starts_process=false sends_prompt=false status_loaded=true report_only=true safe=true source=r26-self-improve-proposal-artifact candidate=2 validated=1 admitted=1 quarantined=1 promoted=1 repair_required=1 starts_daemon=false stops_daemon=false starts_stream=false replays_prompt=false touches_remote=false downloads_model=false warms_model_cache=false"));
        assert!(lines.contains(
            "self_improve_proposal_candidate count=2 ids=proposal-router-guard,proposal-memory-index reason_codes=self_improve_candidate"
        ));
        assert!(lines.contains(
            "self_improve_proposal_repair_required count=1 ids=proposal-needs-fixture reason_codes=missing_fixture"
        ));
        assert!(lines.contains("self_improve_proposal_guidance read_only=true starts_process=false sends_prompt=false status_loaded=true report_only=true safe=true convert_advisory_to_business_evidence=true repair_unvalidated_or_unaccepted=false requires_validation_and_memory_admission=true evidence_ids=round-26:self-improve-proposal-guidance"));
        assert!(lines.contains("self_improve_proposal_action_plan read_only=true starts_process=false sends_prompt=false status_loaded=true report_only=true safe=true action_required=true primary_action=convert_advisory_to_evidence_backed_business_improvement actions=convert_advisory_to_evidence_backed_business_improvement,require_checked_passed_validation_and_accepted_memory_admission requires_validation_and_memory_admission=true auto_apply=false"));
        assert!(lines.contains("self_improve_proposal_action_assignment read_only=true starts_process=false sends_prompt=false status_loaded=true report_only=true safe=true action_required=true primary_action=convert_advisory_to_evidence_backed_business_improvement actions=convert_advisory_to_evidence_backed_business_improvement,require_checked_passed_validation_and_accepted_memory_admission target_count=1 first_target=self-improve-r26-proposal-memory-index first_source_round=26 first_evidence_ids=round-26:self-improve-proposal-guidance first_memory_admission_decision=quarantined first_validation_checked=true first_validation_passed=true first_memory_admission_accepted=false first_evidence_backed_business_improvement=false first_advisory_only=true first_require_repair=false first_missing_requirements=accepted_memory_admission,evidence_backed_business_improvement requires_validation_and_memory_admission=true auto_apply=false"));
        assert!(json.contains("\"status_loaded\":true"));
        assert!(json.contains("\"candidate_count\":2"));
        assert!(json.contains("\"validated_count\":1"));
        assert!(json.contains("\"admitted_count\":1"));
        assert!(json.contains("\"quarantined_count\":1"));
        assert!(json.contains("\"promoted_count\":1"));
        assert!(json.contains("\"repair_required_count\":1"));
        assert!(json.contains("\"starts_daemon\":false"));
        assert!(json.contains("\"starts_stream\":false"));
        assert!(json.contains("\"replays_prompt\":false"));
        assert!(json.contains("\"prompt_guidance\":{"));
        assert!(json.contains("\"action_plan\":{"));
        assert!(json.contains("\"action_assignment\":{"));
        assert!(json.contains("\"action_required\":true"));
        assert!(json.contains(
            "\"primary_action\":\"convert_advisory_to_evidence_backed_business_improvement\""
        ));
        assert!(json.contains("\"target_count\":1"));
        assert!(json.contains("\"first_target\":\"self-improve-r26-proposal-memory-index\""));
        assert!(json.contains("\"first_source_round\":26"));
        assert!(
            json.contains("\"first_evidence_ids\":[\"round-26:self-improve-proposal-guidance\"]")
        );
        assert!(json.contains("\"first_memory_admission_decision\":\"quarantined\""));
        assert!(json.contains("\"first_validation_checked\":true"));
        assert!(json.contains("\"first_validation_passed\":true"));
        assert!(json.contains("\"first_memory_admission_accepted\":false"));
        assert!(json.contains("\"first_evidence_backed_business_improvement\":false"));
        assert!(json.contains("\"first_advisory_only\":true"));
        assert!(json.contains("\"first_require_repair\":false"));
        assert!(json.contains(
            "\"first_missing_requirements\":[\"accepted_memory_admission\",\"evidence_backed_business_improvement\"]"
        ));
        assert!(json.contains("\"auto_apply\":false"));
        assert!(json.contains("\"convert_advisory_to_business_evidence\":true"));
        assert!(json.contains("\"repair_unvalidated_or_unaccepted\":false"));
        assert!(json.contains("\"requires_validation_and_memory_admission\":true"));
        assert!(!json.contains("RAW_OLD_WINDOW_PAYLOAD"));
        assert!(!json.contains("helper says replay prompt"));
        assert!(!json.contains("/v1/chat-stream"));
    }

    #[test]
    fn proposal_panel_accepts_acceptance_summary_prompt_guidance_aliases() {
        let report = r#"{
            "self_improve_proposal_artifact_v1": {
                "schema": "self_improve_proposal_artifact_v1",
                "source": "artifact-without-guidance",
                "read_only": true,
                "starts_process": false,
                "sends_prompt": false,
                "report_only": true,
                "lifecycle": {
                    "candidate": {"count": 1, "ids": ["proposal-a"], "reason_codes": ["candidate"]},
                    "validated": {"count": 0, "ids": [], "reason_codes": []},
                    "admitted": {"count": 0, "ids": [], "reason_codes": []},
                    "quarantined": {"count": 0, "ids": [], "reason_codes": []},
                    "promoted": {"count": 0, "ids": [], "reason_codes": []},
                    "repair_required": {"count": 0, "ids": [], "reason_codes": []}
                }
            },
            "self_improve_proposal_acceptance_summary_v1": {
                "prompt_guidance": {
                    "should_convert_advisory_to_evidence_backed_business_improvement": true,
                    "should_repair_unvalidated_or_unaccepted_proposals": true,
                    "requires_checked_passed_validation_and_accepted_memory_admission": true,
                    "evidence_ids": ["round-55:acceptance-summary"]
                }
            }
        }"#;

        let lines = self_improve_proposal_panel_lines(report).join("\n");
        let json = self_improve_proposal_panel_json(Some(report));

        assert!(lines.contains("self_improve_proposal_guidance read_only=true starts_process=false sends_prompt=false status_loaded=true report_only=true safe=true convert_advisory_to_business_evidence=true repair_unvalidated_or_unaccepted=true requires_validation_and_memory_admission=true evidence_ids=round-55:acceptance-summary"));
        assert!(json.contains("\"convert_advisory_to_business_evidence\":true"));
        assert!(json.contains("\"repair_unvalidated_or_unaccepted\":true"));
        assert!(json.contains("\"requires_validation_and_memory_admission\":true"));
        assert!(json.contains("\"round-55:acceptance-summary\""));
    }

    #[test]
    fn proposal_panel_prefers_acceptance_summary_action_plan() {
        let report = r#"{
            "self_improve_proposal_artifact_v1": {
                "schema": "self_improve_proposal_artifact_v1",
                "source": "artifact-with-stale-action-plan",
                "read_only": true,
                "starts_process": false,
                "sends_prompt": false,
                "report_only": true,
                "action_plan": {
                    "read_only": true,
                    "starts_process": false,
                    "sends_prompt": false,
                    "status_loaded": true,
                    "report_only": true,
                    "safe": true,
                    "action_required": true,
                    "primary_action": "stale_report_action",
                    "actions": ["stale_report_action"],
                    "requires_validation_and_memory_admission": false,
                    "auto_apply": false
                }
            },
            "self_improve_proposal_acceptance_summary_v1": {
                "action_plan": {
                    "read_only": true,
                    "starts_process": false,
                    "sends_prompt": false,
                    "status_loaded": true,
                    "report_only": true,
                    "safe": true,
                    "action_required": true,
                    "primary_action": "convert_advisory_to_evidence_backed_business_improvement",
                    "actions": [
                        "convert_advisory_to_evidence_backed_business_improvement",
                        "require_checked_passed_validation_and_accepted_memory_admission"
                    ],
                    "requires_validation_and_memory_admission": true,
                    "auto_apply": false
                }
            }
        }"#;

        let lines = self_improve_proposal_panel_lines(report).join("\n");
        let json = self_improve_proposal_panel_json(Some(report));

        assert!(lines.contains("self_improve_proposal_action_plan read_only=true starts_process=false sends_prompt=false status_loaded=true report_only=true safe=true action_required=true primary_action=convert_advisory_to_evidence_backed_business_improvement actions=convert_advisory_to_evidence_backed_business_improvement,require_checked_passed_validation_and_accepted_memory_admission requires_validation_and_memory_admission=true auto_apply=false"));
        assert!(!lines.contains("primary_action=stale_report_action"));
        assert!(json.contains(
            "\"primary_action\":\"convert_advisory_to_evidence_backed_business_improvement\""
        ));
        assert!(!json.contains("stale_report_action"));
    }

    #[test]
    fn proposal_panel_projects_acceptance_summary_action_assignment() {
        let report = r#"{
            "self_improve_proposal_artifact_v1": {
                "schema": "self_improve_proposal_artifact_v1",
                "source": "artifact-with-assignment",
                "read_only": true,
                "starts_process": false,
                "sends_prompt": false,
                "report_only": true
            },
            "self_improve_proposal_acceptance_summary_v1": {
                "action_assignment": {
                    "read_only": true,
                    "report_only": true,
                    "candidate_only": true,
                    "auto_apply": false,
                    "action_required": true,
                    "primary_action": "convert_advisory_to_evidence_backed_business_improvement",
                    "actions": [
                        "convert_advisory_to_evidence_backed_business_improvement",
                        "require_checked_passed_validation_and_accepted_memory_admission"
                    ],
                    "target_count": 2,
                    "requires_checked_passed_validation_and_accepted_memory_admission": true,
                    "targets": [
                        {
                            "proposal_id": "self-improve-r385-helper_contract-modifythereviewstagesval",
                            "source_round": 385,
                            "evidence_ids": ["ledger.round.385.helper_stage_contract.review.change_request"],
                            "current_memory_admission_decision": "quarantined",
                            "advisory_only": true,
                            "require_repair": false,
                            "validation_checked": true,
                            "validation_passed": true,
                            "memory_admission_accepted": false,
                            "evidence_backed_business_improvement": false,
                            "missing_requirements": [
                                "accepted_memory_admission",
                                "evidence_backed_business_improvement"
                            ]
                        }
                    ]
                }
            }
        }"#;

        let lines = self_improve_proposal_panel_lines(report).join("\n");
        let json = self_improve_proposal_panel_json(Some(report));

        assert!(lines.contains("self_improve_proposal_action_assignment read_only=true starts_process=false sends_prompt=false status_loaded=true report_only=true safe=true action_required=true primary_action=convert_advisory_to_evidence_backed_business_improvement actions=convert_advisory_to_evidence_backed_business_improvement,require_checked_passed_validation_and_accepted_memory_admission target_count=2 first_target=self-improve-r385-helper_contract-modifythereviewstagesval first_source_round=385 first_evidence_ids=ledger.round.385.helper_stage_contract.review.change_request first_memory_admission_decision=quarantined first_validation_checked=true first_validation_passed=true first_memory_admission_accepted=false first_evidence_backed_business_improvement=false first_advisory_only=true first_require_repair=false first_missing_requirements=accepted_memory_admission,evidence_backed_business_improvement requires_validation_and_memory_admission=true auto_apply=false"));
        assert!(json.contains("\"action_assignment\":{"));
        assert!(json.contains("\"target_count\":2"));
        assert!(json.contains(
            "\"first_target\":\"self-improve-r385-helper_contract-modifythereviewstagesval\""
        ));
        assert!(json.contains("\"first_source_round\":385"));
        assert!(json.contains(
            "\"first_evidence_ids\":[\"ledger.round.385.helper_stage_contract.review.change_request\"]"
        ));
        assert!(json.contains("\"first_memory_admission_decision\":\"quarantined\""));
        assert!(json.contains("\"first_validation_checked\":true"));
        assert!(json.contains("\"first_validation_passed\":true"));
        assert!(json.contains(
            "\"first_missing_requirements\":[\"accepted_memory_admission\",\"evidence_backed_business_improvement\"]"
        ));
    }

    #[test]
    fn proposal_panel_blocks_action_plan_with_side_effects() {
        let report = r#"{
            "self_improve_proposal_artifact_v1": {
                "schema": "self_improve_proposal_artifact_v1",
                "source": "unsafe-action-plan",
                "read_only": true,
                "starts_process": false,
                "sends_prompt": false,
                "report_only": true,
                "action_plan": {
                    "read_only": true,
                    "starts_process": false,
                    "sends_prompt": true,
                    "status_loaded": true,
                    "report_only": true,
                    "safe": true,
                    "action_required": true,
                    "primary_action": "send_prompt_now",
                    "actions": ["send_prompt_now"],
                    "requires_validation_and_memory_admission": false,
                    "auto_apply": true
                }
            }
        }"#;

        let lines = self_improve_proposal_panel_lines(report).join("\n");
        let json = self_improve_proposal_panel_json(Some(report));

        assert!(lines.contains("self_improve_proposal_panel read_only=true starts_process=false sends_prompt=false status_loaded=true report_only=true safe=false"));
        assert!(lines.contains("self_improve_proposal_action_plan read_only=true starts_process=false sends_prompt=false status_loaded=true report_only=true safe=false action_required=true primary_action=send_prompt_now actions=send_prompt_now requires_validation_and_memory_admission=false auto_apply=true"));
        assert!(json.contains("\"safe\":false"));
        assert!(json.contains("\"auto_apply\":true"));
    }

    #[test]
    fn proposal_panel_defaults_to_safe_empty_json_when_absent() {
        let json = self_improve_proposal_panel_json(None);

        assert!(self_improve_proposal_panel_lines("{}").is_empty());
        assert!(json.contains("\"read_only\":true"));
        assert!(json.contains("\"status_loaded\":false"));
        assert!(json.contains("\"safe\":true"));
        assert!(json.contains("\"candidate_count\":0"));
        assert!(json.contains("\"prompt_guidance\":{"));
        assert!(json.contains("\"action_plan\":{"));
        assert!(json.contains("\"action_assignment\":{"));
        assert!(json.contains("\"action_required\":false"));
        assert!(json.contains("\"primary_action\":\"none\""));
        assert!(json.contains("\"convert_advisory_to_business_evidence\":false"));
        assert!(json.contains("\"starts_daemon\":false"));
        assert!(json.contains("\"starts_stream\":false"));
    }
}
