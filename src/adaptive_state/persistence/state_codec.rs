use crate::hierarchy::{
    HierarchyState, HierarchyWeights, ProfileHierarchyObservations, ProfileHierarchyWeights,
};
use crate::router::{ProfileObservations, ProfileThresholds, RouterState};

pub(super) fn serialize_router_state(state: RouterState) -> String {
    format!(
        "{:.6}\t{}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{}\t{}\t{}\t{}",
        state.threshold,
        state.observations,
        state.profile_thresholds.general,
        state.profile_thresholds.coding,
        state.profile_thresholds.writing,
        state.profile_thresholds.long_document,
        state.profile_observations.general,
        state.profile_observations.coding,
        state.profile_observations.writing,
        state.profile_observations.long_document
    )
}

pub(super) fn parse_router_state(value: &str) -> Option<RouterState> {
    let fields = value.split('\t').collect::<Vec<_>>();
    if fields.len() != 2 && fields.len() != 10 {
        return None;
    }

    let threshold = fields[0].parse::<f32>().ok()?;
    let observations = fields[1].parse::<u64>().ok()?;
    let profile_thresholds = if fields.len() == 10 {
        ProfileThresholds {
            general: fields[2].parse::<f32>().ok()?,
            coding: fields[3].parse::<f32>().ok()?,
            writing: fields[4].parse::<f32>().ok()?,
            long_document: fields[5].parse::<f32>().ok()?,
        }
    } else {
        ProfileThresholds::from_single(threshold)
    };
    let profile_observations = if fields.len() == 10 {
        ProfileObservations {
            general: fields[6].parse::<u64>().ok()?,
            coding: fields[7].parse::<u64>().ok()?,
            writing: fields[8].parse::<u64>().ok()?,
            long_document: fields[9].parse::<u64>().ok()?,
        }
    } else {
        ProfileObservations::from_single(observations)
    };

    Some(RouterState {
        threshold,
        observations,
        profile_thresholds,
        profile_observations,
    })
}

pub(super) fn serialize_hierarchy_state(state: HierarchyState) -> String {
    format!(
        "{:.6}\t{:.6}\t{:.6}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        state.current.global,
        state.current.local,
        state.current.convolution,
        serialize_hierarchy_weights(state.profile_weights.general),
        serialize_hierarchy_weights(state.profile_weights.coding),
        serialize_hierarchy_weights(state.profile_weights.writing),
        serialize_hierarchy_weights(state.profile_weights.long_document),
        state.profile_observations.general,
        state.profile_observations.coding,
        state.profile_observations.writing,
        state.profile_observations.long_document
    )
}

pub(super) fn parse_hierarchy_state(value: &str) -> Option<HierarchyState> {
    let fields = value.split('\t').collect::<Vec<_>>();
    if fields.len() != 3 && fields.len() != 19 {
        return None;
    }

    let current = HierarchyWeights::new(
        fields[0].parse::<f32>().ok()?,
        fields[1].parse::<f32>().ok()?,
        fields[2].parse::<f32>().ok()?,
    );
    let profile_weights = if fields.len() == 19 {
        ProfileHierarchyWeights {
            general: parse_hierarchy_weights(&fields[3..6])?,
            coding: parse_hierarchy_weights(&fields[6..9])?,
            writing: parse_hierarchy_weights(&fields[9..12])?,
            long_document: parse_hierarchy_weights(&fields[12..15])?,
        }
    } else {
        ProfileHierarchyWeights::from_single(current)
    };
    let profile_observations = if fields.len() == 19 {
        ProfileHierarchyObservations {
            general: fields[15].parse::<u64>().ok()?,
            coding: fields[16].parse::<u64>().ok()?,
            writing: fields[17].parse::<u64>().ok()?,
            long_document: fields[18].parse::<u64>().ok()?,
        }
    } else {
        ProfileHierarchyObservations::default()
    };

    Some(HierarchyState {
        current,
        profile_weights,
        profile_observations,
    })
}

fn serialize_hierarchy_weights(weights: HierarchyWeights) -> String {
    format!(
        "{:.6}\t{:.6}\t{:.6}",
        weights.global, weights.local, weights.convolution
    )
}

fn parse_hierarchy_weights(fields: &[&str]) -> Option<HierarchyWeights> {
    if fields.len() != 3 {
        return None;
    }

    Some(HierarchyWeights::new(
        fields[0].parse::<f32>().ok()?,
        fields[1].parse::<f32>().ok()?,
        fields[2].parse::<f32>().ok()?,
    ))
}
