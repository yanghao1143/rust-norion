use std::collections::{BTreeMap, BTreeSet};

use crate::{
    ExperienceEnvelope, IndexRebuildPlan, MemoryAdapter, MemoryAdapterCapability,
    MemoryAdapterDescriptor, MemoryAdapterHealth, MemoryResult,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryRepairAction {
    RepairCleanGist,
    CompactContext,
    Quarantine,
    DeleteDuplicate,
}

impl MemoryRepairAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RepairCleanGist => "repair_clean_gist",
            Self::CompactContext => "compact_context",
            Self::Quarantine => "quarantine",
            Self::DeleteDuplicate => "delete_duplicate",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryRepairItem {
    pub experience_id: String,
    pub action: MemoryRepairAction,
    pub reason: String,
    pub proposed_lesson: Option<String>,
    pub source_gist: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryRepairSkippedItem {
    pub experience_id: String,
    pub action: MemoryRepairAction,
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemoryRepairPlan {
    pub items: Vec<MemoryRepairItem>,
    pub skipped: Vec<MemoryRepairSkippedItem>,
}

impl MemoryRepairPlan {
    pub fn is_empty(&self) -> bool {
        self.items.is_empty() && self.skipped.is_empty()
    }

    pub fn items_for_action(&self, action: MemoryRepairAction) -> Vec<&MemoryRepairItem> {
        self.items
            .iter()
            .filter(|item| item.action == action)
            .collect()
    }

    pub fn skipped_for_action(&self, action: MemoryRepairAction) -> Vec<&MemoryRepairSkippedItem> {
        self.skipped
            .iter()
            .filter(|item| item.action == action)
            .collect()
    }

    pub fn reason_codes(&self) -> Vec<String> {
        self.items
            .iter()
            .map(|item| item.reason.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn skipped_reason_codes(&self) -> Vec<String> {
        self.skipped
            .iter()
            .map(|item| item.reason.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn detail_codes(&self) -> Vec<String> {
        self.items
            .iter()
            .map(|item| {
                format!(
                    "{}:{}:{}",
                    item.action.as_str(),
                    item.reason,
                    hex_id(&item.experience_id)
                )
            })
            .chain(self.skipped.iter().map(|item| {
                format!(
                    "skipped:{}:{}:{}",
                    item.action.as_str(),
                    item.reason,
                    hex_id(&item.experience_id)
                )
            }))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn detail_codes_for_action(&self, action: MemoryRepairAction) -> Vec<String> {
        let prefix = format!("{}:", action.as_str());
        self.detail_codes()
            .into_iter()
            .filter(|code| code.starts_with(&prefix))
            .collect()
    }

    pub fn detail_codes_for_reason(&self, reason: &str) -> Vec<String> {
        let needle = format!(":{reason}:");
        self.detail_codes()
            .into_iter()
            .filter(|code| code.contains(&needle))
            .collect()
    }

    pub fn skipped_detail_codes(&self) -> Vec<String> {
        self.detail_codes()
            .into_iter()
            .filter(|code| code.starts_with("skipped:"))
            .collect()
    }

    pub fn skipped_detail_codes_for_action(&self, action: MemoryRepairAction) -> Vec<String> {
        let prefix = format!("skipped:{}:", action.as_str());
        self.detail_codes()
            .into_iter()
            .filter(|code| code.starts_with(&prefix))
            .collect()
    }

    pub fn skipped_detail_codes_for_reason(&self, reason: &str) -> Vec<String> {
        let needle = format!(":{reason}:");
        self.skipped_detail_codes()
            .into_iter()
            .filter(|code| code.contains(&needle))
            .collect()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "memory_repair_plan empty={} items={} skipped={} repair_clean_gist={} compact_context={} quarantine={} delete_duplicate={} skipped_repair_clean_gist={} skipped_compact_context={} skipped_quarantine={} skipped_delete_duplicate={} reason_codes={} skipped_reason_codes={} detail_codes={}",
            self.is_empty(),
            self.items.len(),
            self.skipped.len(),
            self.items_for_action(MemoryRepairAction::RepairCleanGist)
                .len(),
            self.items_for_action(MemoryRepairAction::CompactContext)
                .len(),
            self.items_for_action(MemoryRepairAction::Quarantine).len(),
            self.items_for_action(MemoryRepairAction::DeleteDuplicate)
                .len(),
            self.skipped_for_action(MemoryRepairAction::RepairCleanGist)
                .len(),
            self.skipped_for_action(MemoryRepairAction::CompactContext)
                .len(),
            self.skipped_for_action(MemoryRepairAction::Quarantine)
                .len(),
            self.skipped_for_action(MemoryRepairAction::DeleteDuplicate)
                .len(),
            join_codes(self.reason_codes()),
            join_codes(self.skipped_reason_codes()),
            join_codes(self.detail_codes()),
        )
    }
}

pub trait MemoryRepairPlanner {
    fn plan(&self, records: &[ExperienceEnvelope], rebuild: &IndexRebuildPlan) -> MemoryRepairPlan;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DefaultMemoryRepairPlanner;

impl MemoryAdapter for DefaultMemoryRepairPlanner {
    fn descriptor(&self) -> MemoryAdapterDescriptor {
        MemoryAdapterDescriptor::new(
            "default_memory_repair_planner",
            vec![MemoryAdapterCapability::RepairPlanning],
        )
        .read_only()
    }

    fn health(&self) -> MemoryResult<MemoryAdapterHealth> {
        Ok(MemoryAdapterHealth::ready(None))
    }
}

impl MemoryRepairPlanner for DefaultMemoryRepairPlanner {
    fn plan(&self, records: &[ExperienceEnvelope], rebuild: &IndexRebuildPlan) -> MemoryRepairPlan {
        let by_id = records
            .iter()
            .map(|record| (record.id.as_str(), record))
            .collect::<BTreeMap<_, _>>();
        let duplicate_ids = rebuild
            .deduplicate_groups
            .iter()
            .flat_map(|group| group.duplicate_ids.iter().cloned())
            .collect::<BTreeSet<_>>();
        let missing_clean_gist_ids = rebuild
            .missing_clean_gist_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let dirty_clean_gist_ids = rebuild
            .dirty_clean_gist_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let mut plan = MemoryRepairPlan::default();

        for id in &rebuild.quarantine_candidate_ids {
            push_item_or_skip(
                &mut plan,
                id,
                MemoryRepairAction::Quarantine,
                "governance_quarantine_candidate",
                by_id.get(id.as_str()).copied(),
                None,
            );
        }

        for id in &rebuild.compact_ids {
            push_item_or_skip(
                &mut plan,
                id,
                MemoryRepairAction::CompactContext,
                "compact_long_context_without_gist",
                by_id.get(id.as_str()).copied(),
                None,
            );
        }

        for id in &rebuild.dirty_gist_ids {
            let record = by_id.get(id.as_str()).copied();
            let Some(record) = record else {
                plan.skipped.push(MemoryRepairSkippedItem {
                    experience_id: id.clone(),
                    action: MemoryRepairAction::RepairCleanGist,
                    reason: "missing_record".to_owned(),
                });
                continue;
            };
            let Some(gist) = clean_gist(record) else {
                plan.skipped.push(MemoryRepairSkippedItem {
                    experience_id: id.clone(),
                    action: MemoryRepairAction::RepairCleanGist,
                    reason: clean_gist_skip_reason(id, &dirty_clean_gist_ids).to_owned(),
                });
                continue;
            };
            let proposed_lesson = proposed_lesson(record, &gist);
            plan.items.push(MemoryRepairItem {
                experience_id: id.clone(),
                action: MemoryRepairAction::RepairCleanGist,
                reason: clean_gist_repair_reason(
                    id,
                    &missing_clean_gist_ids,
                    &dirty_clean_gist_ids,
                )
                .to_owned(),
                proposed_lesson: Some(proposed_lesson),
                source_gist: Some(gist),
            });
        }

        let dirty_gist_ids = rebuild
            .dirty_gist_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        for record in records
            .iter()
            .filter(|record| has_metadata_lesson_shape(&record.lesson))
            .filter(|record| !dirty_gist_ids.contains(&record.id))
        {
            let Some(gist) = clean_gist(record) else {
                plan.skipped.push(MemoryRepairSkippedItem {
                    experience_id: record.id.clone(),
                    action: MemoryRepairAction::RepairCleanGist,
                    reason: "missing_clean_gist".to_owned(),
                });
                continue;
            };
            plan.items.push(MemoryRepairItem {
                experience_id: record.id.clone(),
                action: MemoryRepairAction::RepairCleanGist,
                reason: "repair_legacy_metadata_lesson".to_owned(),
                proposed_lesson: Some(proposed_lesson(record, &gist)),
                source_gist: Some(gist),
            });
        }

        for id in duplicate_ids {
            push_item_or_skip(
                &mut plan,
                &id,
                MemoryRepairAction::DeleteDuplicate,
                "deduplicate_exact_fingerprint",
                by_id.get(id.as_str()).copied(),
                None,
            );
        }

        dedup_repair_plan(&mut plan);
        plan
    }
}

fn clean_gist_repair_reason(
    id: &str,
    missing_clean_gist_ids: &BTreeSet<String>,
    dirty_clean_gist_ids: &BTreeSet<String>,
) -> &'static str {
    if dirty_clean_gist_ids.contains(id) {
        "repair_dirty_clean_gist"
    } else if missing_clean_gist_ids.contains(id) {
        "repair_missing_clean_gist"
    } else {
        "repair_missing_or_dirty_clean_gist"
    }
}

fn clean_gist_skip_reason(id: &str, dirty_clean_gist_ids: &BTreeSet<String>) -> &'static str {
    if dirty_clean_gist_ids.contains(id) {
        "dirty_clean_gist"
    } else {
        "missing_clean_gist"
    }
}

fn push_item_or_skip(
    plan: &mut MemoryRepairPlan,
    id: &str,
    action: MemoryRepairAction,
    reason: &str,
    record: Option<&ExperienceEnvelope>,
    proposed_lesson: Option<String>,
) {
    if record.is_some() {
        plan.items.push(MemoryRepairItem {
            experience_id: id.to_owned(),
            action,
            reason: reason.to_owned(),
            proposed_lesson,
            source_gist: None,
        });
    } else {
        plan.skipped.push(MemoryRepairSkippedItem {
            experience_id: id.to_owned(),
            action,
            reason: "missing_record".to_owned(),
        });
    }
}

fn clean_gist(record: &ExperienceEnvelope) -> Option<String> {
    let gist = record.clean_gist.as_deref()?.trim();
    if gist.is_empty()
        || has_transcript_shape(gist)
        || has_metadata_lesson_shape(gist)
        || gist
            .chars()
            .filter(|ch| !ch.is_whitespace() && !ch.is_ascii_punctuation())
            .take(12)
            .count()
            < 12
    {
        return None;
    }
    Some(gist.to_owned())
}

fn proposed_lesson(record: &ExperienceEnvelope, gist: &str) -> String {
    let action = if record
        .lesson
        .trim_start()
        .to_ascii_lowercase()
        .starts_with("rejected_pattern")
    {
        "revise_response"
    } else {
        "reuse_response"
    };
    format!("{action}: {gist}")
}

fn has_transcript_shape(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.contains("conversation transcript:")
        || (value.contains("user:") && value.contains("assistant:"))
}

fn has_metadata_lesson_shape(value: &str) -> bool {
    let value = value.trim().to_ascii_lowercase();
    value.starts_with("accepted_pattern ")
        || value.starts_with("rejected_pattern ")
        || ((value.contains("quality=") || value.contains("overlap="))
            && value.contains("max_severity="))
}

fn dedup_repair_plan(plan: &mut MemoryRepairPlan) {
    let mut seen = BTreeSet::new();
    plan.items
        .retain(|item| seen.insert((item.experience_id.clone(), item.action)));
    let mut skipped_seen = BTreeSet::new();
    plan.skipped
        .retain(|item| skipped_seen.insert((item.experience_id.clone(), item.action)));
    plan.items.sort_by(|left, right| {
        action_rank(left.action)
            .cmp(&action_rank(right.action))
            .then_with(|| left.experience_id.cmp(&right.experience_id))
    });
    plan.skipped.sort_by(|left, right| {
        action_rank(left.action)
            .cmp(&action_rank(right.action))
            .then_with(|| left.experience_id.cmp(&right.experience_id))
    });
}

fn action_rank(action: MemoryRepairAction) -> u8 {
    match action {
        MemoryRepairAction::Quarantine => 0,
        MemoryRepairAction::DeleteDuplicate => 1,
        MemoryRepairAction::CompactContext => 2,
        MemoryRepairAction::RepairCleanGist => 3,
    }
}

fn join_codes(codes: Vec<String>) -> String {
    if codes.is_empty() {
        "none".to_owned()
    } else {
        codes.join("|")
    }
}

fn hex_id(id: &str) -> String {
    id.as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DefaultExperienceGovernance, ExperienceGovernance, MemoryScope};

    #[test]
    fn repair_plan_projects_governance_actions() {
        let mut repairable = ExperienceEnvelope::new(
            "repairable",
            "runtime prompt",
            "accepted_pattern quality=0.9 max_severity=watch",
        )
        .with_clean_gist("Use the scoped memory gate before injecting recall context.");
        repairable.quality = 0.6;
        let polluted = ExperienceEnvelope::new(
            "polluted",
            "Conversation Transcript:\nUser: ssh -o ConnectTimeout=1 gitlab.local\nAssistant: ok",
            "accepted_pattern quality=0.1 max_severity=critical",
        )
        .with_scope(MemoryScope::for_task("ops"));
        let duplicate_a = ExperienceEnvelope::new("dupe-a", "same", "lesson");
        let duplicate_b = ExperienceEnvelope::new("dupe-b", "same", "lesson");
        let records = vec![repairable, polluted, duplicate_a, duplicate_b];
        let rebuild = DefaultExperienceGovernance::default().rebuild_plan(&records);

        let plan = DefaultMemoryRepairPlanner.plan(&records, &rebuild);
        assert!(
            plan.items_for_action(MemoryRepairAction::Quarantine)
                .iter()
                .any(|item| item.experience_id == "polluted")
        );
        assert!(
            plan.items_for_action(MemoryRepairAction::DeleteDuplicate)
                .iter()
                .any(|item| item.experience_id == "dupe-b")
        );
        let clean_gist_items = plan.items_for_action(MemoryRepairAction::RepairCleanGist);
        assert!(clean_gist_items.iter().any(|item| {
            item.experience_id == "repairable"
                && item
                    .proposed_lesson
                    .as_deref()
                    .is_some_and(|lesson| lesson.starts_with("reuse_response:"))
        }));
        assert_eq!(
            plan.summary_line(),
            "memory_repair_plan empty=false items=3 skipped=1 repair_clean_gist=1 compact_context=0 quarantine=1 delete_duplicate=1 skipped_repair_clean_gist=1 skipped_compact_context=0 skipped_quarantine=0 skipped_delete_duplicate=0 reason_codes=deduplicate_exact_fingerprint|governance_quarantine_candidate|repair_legacy_metadata_lesson skipped_reason_codes=missing_clean_gist detail_codes=delete_duplicate:deduplicate_exact_fingerprint:647570652d62|quarantine:governance_quarantine_candidate:706f6c6c75746564|repair_clean_gist:repair_legacy_metadata_lesson:72657061697261626c65|skipped:repair_clean_gist:missing_clean_gist:706f6c6c75746564"
        );
        assert_eq!(
            plan.reason_codes(),
            vec![
                "deduplicate_exact_fingerprint".to_owned(),
                "governance_quarantine_candidate".to_owned(),
                "repair_legacy_metadata_lesson".to_owned(),
            ]
        );
        assert_eq!(
            plan.skipped_reason_codes(),
            vec!["missing_clean_gist".to_owned()]
        );
        assert_eq!(
            plan.detail_codes(),
            vec![
                "delete_duplicate:deduplicate_exact_fingerprint:647570652d62".to_owned(),
                "quarantine:governance_quarantine_candidate:706f6c6c75746564".to_owned(),
                "repair_clean_gist:repair_legacy_metadata_lesson:72657061697261626c65".to_owned(),
                "skipped:repair_clean_gist:missing_clean_gist:706f6c6c75746564".to_owned(),
            ]
        );
        assert_eq!(
            plan.detail_codes_for_action(MemoryRepairAction::DeleteDuplicate),
            vec!["delete_duplicate:deduplicate_exact_fingerprint:647570652d62".to_owned()]
        );
        assert_eq!(
            plan.detail_codes_for_action(MemoryRepairAction::RepairCleanGist),
            vec!["repair_clean_gist:repair_legacy_metadata_lesson:72657061697261626c65".to_owned()]
        );
        assert_eq!(
            plan.detail_codes_for_reason("missing_clean_gist"),
            vec!["skipped:repair_clean_gist:missing_clean_gist:706f6c6c75746564".to_owned()]
        );
        assert_eq!(
            plan.skipped_detail_codes(),
            vec!["skipped:repair_clean_gist:missing_clean_gist:706f6c6c75746564".to_owned()]
        );
        assert_eq!(
            plan.skipped_detail_codes_for_action(MemoryRepairAction::RepairCleanGist),
            vec!["skipped:repair_clean_gist:missing_clean_gist:706f6c6c75746564".to_owned()]
        );
        assert_eq!(
            plan.skipped_detail_codes_for_reason("missing_clean_gist"),
            vec!["skipped:repair_clean_gist:missing_clean_gist:706f6c6c75746564".to_owned()]
        );
    }

    #[test]
    fn repair_plan_skips_missing_clean_gist() {
        let record = ExperienceEnvelope::new(
            "legacy",
            "runtime prompt",
            "accepted_pattern quality=0.9 max_severity=watch",
        );
        let rebuild =
            DefaultExperienceGovernance::default().rebuild_plan(std::slice::from_ref(&record));

        let plan = DefaultMemoryRepairPlanner.plan(&[record], &rebuild);
        assert!(
            plan.items_for_action(MemoryRepairAction::RepairCleanGist)
                .is_empty()
        );
        assert_eq!(
            plan.skipped_for_action(MemoryRepairAction::RepairCleanGist)[0].reason,
            "missing_clean_gist"
        );
        assert_eq!(
            plan.summary_line(),
            "memory_repair_plan empty=false items=0 skipped=1 repair_clean_gist=0 compact_context=0 quarantine=0 delete_duplicate=0 skipped_repair_clean_gist=1 skipped_compact_context=0 skipped_quarantine=0 skipped_delete_duplicate=0 reason_codes=none skipped_reason_codes=missing_clean_gist detail_codes=skipped:repair_clean_gist:missing_clean_gist:6c6567616379"
        );
        assert_eq!(
            plan.detail_codes(),
            vec!["skipped:repair_clean_gist:missing_clean_gist:6c6567616379".to_owned()]
        );
        assert_eq!(
            plan.skipped_detail_codes_for_reason("missing_clean_gist"),
            vec!["skipped:repair_clean_gist:missing_clean_gist:6c6567616379".to_owned()]
        );
    }

    #[test]
    fn repair_plan_rejects_dirty_gist_sources() {
        let record = ExperienceEnvelope::new(
            "dirty",
            "runtime prompt",
            "accepted_pattern quality=0.9 max_severity=watch",
        )
        .with_clean_gist("Conversation Transcript: User: stale Assistant: stale");
        let rebuild =
            DefaultExperienceGovernance::default().rebuild_plan(std::slice::from_ref(&record));

        let plan = DefaultMemoryRepairPlanner.plan(&[record], &rebuild);
        assert_eq!(
            plan.skipped_for_action(MemoryRepairAction::RepairCleanGist)[0].reason,
            "dirty_clean_gist"
        );
        assert_eq!(
            plan.skipped_reason_codes(),
            vec!["dirty_clean_gist".to_owned()]
        );
        assert_eq!(
            plan.detail_codes(),
            vec!["skipped:repair_clean_gist:dirty_clean_gist:6469727479".to_owned()]
        );
        assert_eq!(
            plan.skipped_detail_codes_for_reason("dirty_clean_gist"),
            vec!["skipped:repair_clean_gist:dirty_clean_gist:6469727479".to_owned()]
        );
    }

    #[test]
    fn repair_plan_preserves_missing_vs_dirty_clean_gist_reasons() {
        let missing_now_repaired = ExperienceEnvelope::new(
            "missing",
            "runtime prompt",
            "accepted_pattern quality=0.9 max_severity=watch",
        )
        .with_clean_gist("Summarize the runtime repair as a scoped reusable lesson.");
        let dirty_now_repaired = ExperienceEnvelope::new(
            "dirty",
            "runtime prompt",
            "accepted_pattern quality=0.9 max_severity=watch",
        )
        .with_clean_gist("Summarize the adapter cleanup as a scoped reusable lesson.");
        let rebuild = IndexRebuildPlan {
            missing_clean_gist_ids: vec!["missing".to_owned()],
            dirty_clean_gist_ids: vec!["dirty".to_owned()],
            dirty_gist_ids: vec!["dirty".to_owned(), "missing".to_owned()],
            reasons: vec!["repair_missing_or_dirty_clean_gist".to_owned()],
            rebuild_required: true,
            ..IndexRebuildPlan::default()
        };

        let plan =
            DefaultMemoryRepairPlanner.plan(&[missing_now_repaired, dirty_now_repaired], &rebuild);

        assert_eq!(
            plan.reason_codes(),
            vec![
                "repair_dirty_clean_gist".to_owned(),
                "repair_missing_clean_gist".to_owned(),
            ]
        );
        assert_eq!(
            plan.detail_codes(),
            vec![
                "repair_clean_gist:repair_dirty_clean_gist:6469727479".to_owned(),
                "repair_clean_gist:repair_missing_clean_gist:6d697373696e67".to_owned(),
            ]
        );
        assert_eq!(
            plan.detail_codes_for_action(MemoryRepairAction::RepairCleanGist),
            vec![
                "repair_clean_gist:repair_dirty_clean_gist:6469727479".to_owned(),
                "repair_clean_gist:repair_missing_clean_gist:6d697373696e67".to_owned(),
            ]
        );
        assert_eq!(
            plan.detail_codes_for_reason("repair_missing_clean_gist"),
            vec!["repair_clean_gist:repair_missing_clean_gist:6d697373696e67".to_owned()]
        );
        assert_eq!(plan.skipped_detail_codes(), Vec::<String>::new());
    }

    #[test]
    fn repair_planner_is_read_only_adapter() {
        let descriptor = DefaultMemoryRepairPlanner.descriptor();
        assert_eq!(descriptor.name, "default_memory_repair_planner");
        assert!(descriptor.read_only);
        assert!(
            descriptor
                .capabilities
                .contains(&MemoryAdapterCapability::RepairPlanning)
        );
    }
}
