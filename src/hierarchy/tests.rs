use super::*;
use crate::router::GenerationMetrics;

#[test]
fn coding_profile_prefers_local_attention() {
    let target = HierarchyController::target_for_profile(TaskProfile::Coding);

    assert!(target.local > target.global);
    assert!(target.local > target.convolution);
}

#[test]
fn weights_are_normalized() {
    let weights = HierarchyWeights::new(10.0, 5.0, 1.0);
    let sum = weights.global + weights.local + weights.convolution;

    assert!((sum - 1.0).abs() < 0.0001);
}

#[test]
fn observations_update_only_selected_profile_weights() {
    let mut controller = HierarchyController::new();
    let coding_before = controller.state().profile_weights.get(TaskProfile::Coding);
    let writing_before = controller.state().profile_weights.get(TaskProfile::Writing);

    controller.observe(
        TaskProfile::Writing,
        GenerationMetrics {
            perplexity: 30.0,
            semantic_consistency: 0.2,
            contradiction_count: 2,
            token_count: 32,
        },
    );

    let state = controller.state();
    let coding_after = state.profile_weights.get(TaskProfile::Coding);
    let writing_after = state.profile_weights.get(TaskProfile::Writing);
    assert!((coding_after.local - coding_before.local).abs() < 0.0001);
    assert!(writing_after.global > writing_before.global);
    assert_eq!(state.profile_observations.get(TaskProfile::Writing), 1);
    assert_eq!(state.profile_observations.get(TaskProfile::Coding), 0);
}

#[test]
fn adapt_to_profile_uses_profile_specific_learned_weights() {
    let mut controller = HierarchyController::new();
    controller.observe(
        TaskProfile::LongDocument,
        GenerationMetrics {
            perplexity: 32.0,
            semantic_consistency: 0.2,
            contradiction_count: 1,
            token_count: 64,
        },
    );
    let learned_long = controller
        .state()
        .profile_weights
        .get(TaskProfile::LongDocument);

    let adapted_coding = controller.adapt_to_profile(TaskProfile::Coding);
    let adapted_long = controller.adapt_to_profile(TaskProfile::LongDocument);

    assert!(adapted_coding.local > adapted_coding.convolution);
    assert!(learned_long.convolution > adapted_coding.convolution);
    assert!(adapted_long.convolution > adapted_coding.convolution);
}
