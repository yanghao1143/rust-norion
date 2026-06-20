use super::controller;
use super::profile::TaskProfile;
use super::weights::HierarchyWeights;

#[derive(Debug, Clone, Copy)]
pub struct ProfileHierarchyWeights {
    pub general: HierarchyWeights,
    pub coding: HierarchyWeights,
    pub writing: HierarchyWeights,
    pub long_document: HierarchyWeights,
}

impl ProfileHierarchyWeights {
    pub fn target_defaults() -> Self {
        Self {
            general: controller::target_for_profile(TaskProfile::General),
            coding: controller::target_for_profile(TaskProfile::Coding),
            writing: controller::target_for_profile(TaskProfile::Writing),
            long_document: controller::target_for_profile(TaskProfile::LongDocument),
        }
    }

    pub fn from_single(weights: HierarchyWeights) -> Self {
        Self {
            general: weights,
            coding: weights,
            writing: weights,
            long_document: weights,
        }
    }

    pub fn get(self, profile: TaskProfile) -> HierarchyWeights {
        match profile {
            TaskProfile::General => self.general,
            TaskProfile::Coding => self.coding,
            TaskProfile::Writing => self.writing,
            TaskProfile::LongDocument => self.long_document,
        }
    }

    pub fn set(&mut self, profile: TaskProfile, weights: HierarchyWeights) {
        match profile {
            TaskProfile::General => self.general = weights,
            TaskProfile::Coding => self.coding = weights,
            TaskProfile::Writing => self.writing = weights,
            TaskProfile::LongDocument => self.long_document = weights,
        }
    }

    pub fn normalize(&mut self) {
        self.general.normalize();
        self.coding.normalize();
        self.writing.normalize();
        self.long_document.normalize();
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ProfileHierarchyObservations {
    pub general: u64,
    pub coding: u64,
    pub writing: u64,
    pub long_document: u64,
}

impl ProfileHierarchyObservations {
    pub fn from_single(observations: u64) -> Self {
        Self {
            general: observations,
            coding: 0,
            writing: 0,
            long_document: 0,
        }
    }

    pub fn get(self, profile: TaskProfile) -> u64 {
        match profile {
            TaskProfile::General => self.general,
            TaskProfile::Coding => self.coding,
            TaskProfile::Writing => self.writing,
            TaskProfile::LongDocument => self.long_document,
        }
    }

    pub fn bump(&mut self, profile: TaskProfile) {
        match profile {
            TaskProfile::General => self.general = self.general.saturating_add(1),
            TaskProfile::Coding => self.coding = self.coding.saturating_add(1),
            TaskProfile::Writing => self.writing = self.writing.saturating_add(1),
            TaskProfile::LongDocument => {
                self.long_document = self.long_document.saturating_add(1);
            }
        }
    }
}
