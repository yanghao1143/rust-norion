use crate::hierarchy::TaskProfile;
use crate::reflection::DraftToken;
use crate::router::{NoironRouter, Route};

use super::TokenStreamMonitor;

#[test]
fn stable_stream_updates_router_per_window() {
    let mut router = NoironRouter::new();
    let monitor = TokenStreamMonitor::new(4);
    let before = router.threshold();
    let reports = monitor.observe_generated(
        &mut router,
        "alpha beta gamma delta epsilon zeta eta theta",
        0.99,
        0,
    );

    assert_eq!(reports.len(), 2);
    assert_eq!(router.observations(), 2);
    assert!(router.threshold() >= before);
}

#[test]
fn weak_stream_lowers_threshold() {
    let mut router = NoironRouter::new();
    let monitor = TokenStreamMonitor::new(4);
    let before = router.threshold();
    monitor.observe_generated(
        &mut router,
        "uncertain contradiction maybe unstable output",
        0.1,
        2,
    );

    assert!(router.threshold() < before);
}

#[test]
fn runtime_tokens_use_supplied_entropy() {
    let mut router = NoironRouter::new();
    let monitor = TokenStreamMonitor::new(2);
    let tokens = vec![
        DraftToken {
            text: "easy".to_owned(),
            logprob: Some(-0.1),
            entropy: Some(0.1),
        },
        DraftToken {
            text: "hard".to_owned(),
            logprob: Some(-1.2),
            entropy: Some(0.9),
        },
    ];

    let reports = monitor.observe_tokens(&mut router, &tokens, 0.95, 0);

    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].observations[0].route, Route::FastProjection);
    assert!(reports[0].observations[1].route.uses_attention_budget());
}

#[test]
fn stream_feedback_updates_selected_profile() {
    let mut router = NoironRouter::new();
    let monitor = TokenStreamMonitor::new(4);

    monitor.observe_generated_with_profile(
        &mut router,
        TaskProfile::Coding,
        "fn main value compute branch return",
        0.97,
        0,
    );

    let state = router.state();
    assert_eq!(state.profile_observations.get(TaskProfile::General), 0);
    assert_eq!(state.profile_observations.get(TaskProfile::Coding), 2);
}
