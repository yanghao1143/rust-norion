use crate::hierarchy::TaskProfile;
use crate::runtime::RuntimeRequest;
use crate::runtime_manifest::RuntimeManifest;
use crate::transformer::TransformerPlanCounts;

use super::super::forward::{LocalForwardState, count_forward_layers};

pub(super) struct ResponseEvidence {
    pub(super) transformer_counts: TransformerPlanCounts,
    pub(super) profile_hint: &'static str,
    pub(super) selected_adapter: Option<String>,
}

pub(super) fn collect_response_evidence(
    request: &RuntimeRequest,
    manifest: &RuntimeManifest,
    forward: &LocalForwardState,
) -> ResponseEvidence {
    ResponseEvidence {
        transformer_counts: count_forward_layers(&forward.layer_summaries),
        profile_hint: profile_hint(request.profile),
        selected_adapter: manifest
            .preferred_adapter_with_observations(
                &request.hardware_plan.execution,
                &request.runtime_adapter_observations,
            )
            .map(|adapter| adapter.as_str().to_owned()),
    }
}

fn profile_hint(profile: TaskProfile) -> &'static str {
    match profile {
        TaskProfile::General => "balanced reasoning",
        TaskProfile::Coding => "local-window syntax and interface tracking",
        TaskProfile::Writing => "global continuity and style preservation",
        TaskProfile::LongDocument => "convolutional compression plus global memory recall",
    }
}
