use crate::engine::InferenceOutcome;
use crate::hardware::DeviceClass;

use super::{explicit_device_count, push_unique_device};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct BenchmarkReflectionEvidence {
    pub issue_cases: usize,
    pub total_issues: usize,
    pub critical_issue_cases: usize,
    pub total_critical_issues: usize,
    pub revision_action_cases: usize,
    pub total_revision_actions: usize,
    pub live_memory_feedback_reinforcements: usize,
    pub live_memory_feedback_penalties: usize,
    pub live_memory_feedback_applied: usize,
    pub live_memory_feedback_removed: usize,
    pub live_memory_feedback_missing: usize,
    pub live_memory_feedback_strength_delta: f32,
    pub memory_feedback_failures: Vec<String>,
    pub(super) issue_devices: Vec<DeviceClass>,
    pub(super) critical_issue_devices: Vec<DeviceClass>,
    pub(super) revision_action_devices: Vec<DeviceClass>,
}

impl BenchmarkReflectionEvidence {
    pub(super) fn record(&mut self, outcome: &InferenceOutcome) {
        let issues = outcome.report.issues.len();
        let critical_issues = outcome.report.critical_issue_count();
        let revision_actions = outcome.report.revision_actions.len();

        self.issue_cases += usize::from(issues > 0);
        self.total_issues += issues;
        self.critical_issue_cases += usize::from(critical_issues > 0);
        self.total_critical_issues += critical_issues;
        self.revision_action_cases += usize::from(revision_actions > 0);
        self.total_revision_actions += revision_actions;
        self.live_memory_feedback_reinforcements += outcome.memory_feedback.reinforced;
        self.live_memory_feedback_penalties += outcome.memory_feedback.penalized;
        self.live_memory_feedback_applied += outcome.memory_feedback.applied_updates();
        self.live_memory_feedback_removed += outcome.memory_feedback.removed_updates();
        self.live_memory_feedback_missing += outcome.memory_feedback.missing_updates();
        self.live_memory_feedback_strength_delta += outcome.memory_feedback.strength_delta();
        let expected_updates = outcome
            .memory_feedback
            .reinforced
            .saturating_add(outcome.memory_feedback.penalized);
        if outcome.memory_feedback.updates.len() != expected_updates {
            self.memory_feedback_failures.push(format!(
                "{}:{} memory feedback update reports {} do not match reinforced+penalized {}",
                outcome.hardware_plan.device.as_str(),
                outcome.experience_id,
                outcome.memory_feedback.updates.len(),
                expected_updates
            ));
        }
        if outcome
            .memory_feedback
            .applied_updates()
            .saturating_add(outcome.memory_feedback.missing_updates())
            != expected_updates
        {
            self.memory_feedback_failures.push(format!(
                "{}:{} memory feedback applied+missing {} does not match updates {}",
                outcome.hardware_plan.device.as_str(),
                outcome.experience_id,
                outcome
                    .memory_feedback
                    .applied_updates()
                    .saturating_add(outcome.memory_feedback.missing_updates()),
                expected_updates
            ));
        }
        if outcome.memory_feedback.removed_updates() > outcome.memory_feedback.applied_updates() {
            self.memory_feedback_failures.push(format!(
                "{}:{} memory feedback removed {} exceeds applied {}",
                outcome.hardware_plan.device.as_str(),
                outcome.experience_id,
                outcome.memory_feedback.removed_updates(),
                outcome.memory_feedback.applied_updates()
            ));
        }
        if outcome.memory_feedback.total_updates() > 0
            && outcome.memory_feedback.applied_updates() == 0
            && outcome.memory_feedback.missing_updates() == 0
        {
            self.memory_feedback_failures.push(format!(
                "{}:{} memory feedback has updates but no applied/missing evidence",
                outcome.hardware_plan.device.as_str(),
                outcome.experience_id
            ));
        }

        let device = outcome.hardware_plan.device;
        if issues > 0 {
            push_unique_device(&mut self.issue_devices, device);
        }
        if critical_issues > 0 {
            push_unique_device(&mut self.critical_issue_devices, device);
        }
        if revision_actions > 0 {
            push_unique_device(&mut self.revision_action_devices, device);
        }
    }

    pub fn issue_device_profiles(&self) -> usize {
        explicit_device_count(&self.issue_devices)
    }

    pub fn critical_issue_device_profiles(&self) -> usize {
        explicit_device_count(&self.critical_issue_devices)
    }

    pub fn revision_action_device_profiles(&self) -> usize {
        explicit_device_count(&self.revision_action_devices)
    }

    pub fn live_memory_feedback_updates(&self) -> usize {
        self.live_memory_feedback_reinforcements + self.live_memory_feedback_penalties
    }

    pub fn memory_feedback_evidence_failures(&self) -> usize {
        self.memory_feedback_failures.len()
    }
}
