use super::*;

#[test]
fn blocks_expansion_when_quality_is_down() {
    let advice = model_pool_advice(
        "SmartSteam model pool status\nquality_ready=false\nquality_context_sufficient=false\ncapacity policy=one_quality_plus_small_helpers expansion_allowed=false recommendation=restore_quality_gate_first healthy_helper_worker_count=0\nworker role=quality status=unreachable ready=false runtime_device=metal runtime_accelerator=metal gpu_layers=99",
    );

    assert!(advice.contains("safe_to_enable_pool_workers=false"));
    assert!(advice.contains("next_step=start_or_fix_quality_worker_8686"));
    assert!(advice.contains("section=advice_json"));
    assert!(advice.contains("\"schema\":\"smartsteam.forge.model_pool_advice.v1\""));
    assert!(advice.contains("\"safe_to_enable_pool_workers\":false"));
    assert!(advice.contains("capacity_policy=one_quality_plus_small_helpers"));
    assert!(advice.contains("avoid_extra_12b=true"));
    assert!(advice.contains("max_quality_12b_workers=1"));
    assert!(advice.contains("expected_helper_roles=summary,router,review,index,test-gate"));
    assert!(advice.contains("missing_helper_roles=summary,router,review,index,test-gate"));
    assert!(advice.contains("helper_cpu_or_no_gpu_roles=none"));
    assert!(advice.contains("parallel_worker_shape=quality:1 helpers_visible:0 helper_target:5"));
}

#[test]
fn asks_for_metal_fix_before_expansion() {
    let advice = model_pool_advice(
        "SmartSteam model pool status\nquality_ready=true\nquality_context_sufficient=true\ncapacity policy=one_quality_plus_small_helpers expansion_allowed=true recommendation=add_summary_worker_first healthy_helper_worker_count=0\nworker role=quality status=healthy ready=true runtime_device=cpu runtime_accelerator=cpu gpu_layers=0",
    );

    assert!(advice.contains("safe_to_enable_pool_workers=false"));
    assert!(advice.contains("next_step=fix_quality_metal_or_gpu_layers_before_expansion"));
    assert!(advice.contains("reason=quality_worker_not_gpu_accelerated"));
}

#[test]
fn recommends_summary_first_after_quality_is_ready() {
    let advice = model_pool_advice(
        "SmartSteam model pool status\nquality_ready=true\nquality_context_sufficient=true\nquality_context_tokens=262144\nquality_context_required_tokens=262144\ncapacity policy=one_quality_plus_small_helpers expansion_allowed=true recommendation=add_summary_worker_first healthy_helper_worker_count=0 quality_runtime_accelerated=true\nworker role=quality status=healthy ready=true runtime_device=metal runtime_accelerator=metal gpu_layers=99",
    );

    assert!(advice.contains("safe_to_enable_pool_workers=true"));
    assert!(advice.contains("next_step=add_summary_worker_first"));
    assert!(advice.contains("quality_runtime_accelerated=true"));
    assert!(advice.contains("\"safe_to_enable_pool_workers\":true"));
    assert!(advice.contains("\"next_step\":\"add_summary_worker_first\""));
    assert!(
        advice.contains("recommended_launch_order=quality,summary,router,review,index,test-gate")
    );
}

#[test]
fn advice_report_json_summary_matches_text_decision_surface() {
    let advice = model_pool_advice(
        "SmartSteam model pool status\nquality_ready=true\nquality_context_sufficient=true\nquality_context_tokens=262144\nquality_context_required_tokens=262144\ncapacity policy=one_quality_plus_small_helpers expansion_allowed=true recommendation=add_summary_worker_first healthy_helper_worker_count=0 quality_runtime_accelerated=true\nworker role=quality status=healthy ready=true runtime_device=metal runtime_accelerator=metal gpu_layers=99",
    );
    let advice_json = section_body(&advice, "section=advice_json")
        .expect("advice report should include advice_json body");

    let summary = super::json::model_pool_advice_json_summary(advice_json).unwrap();

    assert!(summary.safe_to_enable_pool_workers);
    assert_eq!(summary.next_step, "add_summary_worker_first");
    assert_eq!(summary.reason, "quality_chain_ready_no_helpers_visible");
    assert_eq!(summary.kind, "busy");
    assert_eq!(
        summary.missing_helper_roles,
        vec![
            "summary".to_owned(),
            "router".to_owned(),
            "review".to_owned(),
            "index".to_owned(),
            "test-gate".to_owned()
        ]
    );
    assert!(advice.contains("safe_to_enable_pool_workers=true"));
    assert!(advice.contains("next_step=add_summary_worker_first"));
    assert!(advice.contains("reason=quality_chain_ready_no_helpers_visible"));
}

#[test]
fn advice_report_validator_accepts_text_and_json_contract() {
    let advice = model_pool_advice(
        "SmartSteam model pool status\nquality_ready=true\nquality_context_sufficient=true\nquality_context_tokens=262144\nquality_context_required_tokens=262144\ncapacity policy=one_quality_plus_small_helpers expansion_allowed=true recommendation=add_summary_worker_first healthy_helper_worker_count=0 quality_runtime_accelerated=true\nworker role=quality status=healthy ready=true runtime_device=metal runtime_accelerator=metal gpu_layers=99",
    );

    validate_model_pool_advice_report(&advice).unwrap();
}

#[test]
fn advice_report_validator_rejects_text_json_decision_drift() {
    let advice = model_pool_advice(
        "SmartSteam model pool status\nquality_ready=true\nquality_context_sufficient=true\nquality_context_tokens=262144\nquality_context_required_tokens=262144\ncapacity policy=one_quality_plus_small_helpers expansion_allowed=true recommendation=add_summary_worker_first healthy_helper_worker_count=0 quality_runtime_accelerated=true\nworker role=quality status=healthy ready=true runtime_device=metal runtime_accelerator=metal gpu_layers=99",
    )
    .replacen(
        "next_step=add_summary_worker_first",
        "next_step=start_or_fix_quality_worker_8686",
        1,
    );

    assert!(
        validate_model_pool_advice_report(&advice)
            .unwrap_err()
            .contains("model pool advice text missing")
    );
}

#[test]
fn advice_report_validator_rejects_missing_advice_json() {
    let advice = model_pool_advice(
        "SmartSteam model pool status\nquality_ready=true\nquality_context_sufficient=true\nquality_context_tokens=262144\nquality_context_required_tokens=262144\ncapacity policy=one_quality_plus_small_helpers expansion_allowed=true recommendation=add_summary_worker_first healthy_helper_worker_count=0 quality_runtime_accelerated=true\nworker role=quality status=healthy ready=true runtime_device=metal runtime_accelerator=metal gpu_layers=99",
    )
    .replace("section=advice_json", "section=missing_json");

    assert!(
        validate_model_pool_advice_report(&advice)
            .unwrap_err()
            .contains("model pool advice missing section=advice_json")
    );
}

#[test]
fn recommends_remaining_roles_for_partial_pool() {
    let advice = model_pool_advice(
        "SmartSteam model pool status\nquality_ready=true\nquality_context_sufficient=true\ncapacity policy=one_quality_plus_small_helpers expansion_allowed=true recommendation=add_review_or_index_worker_after_short_smoke healthy_helper_worker_count=2 quality_runtime_accelerated=true\nworker role=quality status=healthy ready=true runtime_device=metal runtime_accelerator=metal gpu_layers=99\nworker role=summary status=healthy ready=true runtime_device=metal runtime_accelerator=metal gpu_layers=99\nworker role=review status=healthy ready=true runtime_device=metal runtime_accelerator=metal gpu_layers=99",
    );

    assert!(advice.contains(
        "helper_roles=summary:true router:false review:true test-gate:false index:false"
    ));
    assert!(advice.contains("expected_helper_roles=summary,router,review,index,test-gate"));
    assert!(advice.contains("missing_helper_roles=router,index,test-gate"));
    assert!(advice.contains("parallel_worker_shape=quality:1 helpers_visible:2 helper_target:5"));
    assert!(advice.contains("next_step=add_remaining_helper_roles_one_at_a_time"));
}

#[test]
fn treats_summary_and_test_gate_as_partial_pool() {
    let advice = model_pool_advice(
        "SmartSteam model pool status\nquality_ready=true\nquality_context_sufficient=true\ncapacity policy=one_quality_plus_small_helpers expansion_allowed=true recommendation=add_remaining_helper_roles_one_at_a_time healthy_helper_worker_count=2 quality_runtime_accelerated=true\nworker role=quality status=healthy ready=true runtime_device=metal runtime_accelerator=metal gpu_layers=99\nworker role=summary status=healthy ready=true runtime_device=metal runtime_accelerator=metal gpu_layers=99\nworker role=test-gate status=healthy ready=true runtime_device=metal runtime_accelerator=metal gpu_layers=80",
    );

    assert!(advice.contains(
        "helper_roles=summary:true router:false review:false test-gate:true index:false"
    ));
    assert!(advice.contains("missing_helper_roles=router,review,index"));
    assert!(advice.contains("next_step=add_remaining_helper_roles_one_at_a_time"));
    assert!(advice.contains("reason=partial_helper_pool_visible"));
}

#[test]
fn blocks_pool_expansion_when_helper_workers_are_cpu_or_no_gpu() {
    let advice = model_pool_advice(
        "SmartSteam model pool status\nquality_ready=true\nquality_context_sufficient=true\ncapacity policy=one_quality_plus_small_helpers expansion_allowed=true recommendation=add_remaining_helper_roles_one_at_a_time healthy_helper_worker_count=2 quality_runtime_accelerated=true\nworker role=quality status=healthy ready=true runtime_device=metal runtime_accelerator=metal gpu_layers=99\nworker role=summary status=healthy ready=true runtime_device=metal runtime_accelerator=metal gpu_layers=80\nworker role=review status=healthy ready=true runtime_device=cpu runtime_accelerator=none gpu_layers=0",
    );

    assert!(advice.contains("safe_to_enable_pool_workers=false"));
    assert!(advice.contains("next_step=fix_helper_metal_or_gpu_layers_before_more_pool_workers"));
    assert!(advice.contains("reason=helper_workers_not_gpu_accelerated"));
    assert!(advice.contains("helper_cpu_or_no_gpu_roles=review"));
    assert!(advice.contains("\"helper_cpu_or_no_gpu_roles\":[\"review\"]"));
}

#[test]
fn blocks_extra_quality_12b_workers_on_shared_apple_memory() {
    let advice = model_pool_advice(
        "SmartSteam model pool status\nquality_ready=true\nquality_context_sufficient=true\ncapacity policy=one_quality_plus_small_helpers expansion_allowed=true recommendation=add_summary_worker_first healthy_helper_worker_count=1 quality_runtime_accelerated=true\nworker role=quality status=healthy ready=true runtime_device=metal runtime_accelerator=metal gpu_layers=99\nworker role=quality status=healthy ready=true runtime_device=metal runtime_accelerator=metal gpu_layers=99\nworker role=summary status=healthy ready=true runtime_device=metal runtime_accelerator=metal gpu_layers=80",
    );

    assert!(advice.contains("extra_quality_12b_detected=true"));
    assert!(advice.contains("safe_to_enable_pool_workers=false"));
    assert!(
        advice.contains("next_step=stop_extra_quality_12b_workers_keep_one_quality_plus_helpers")
    );
    assert!(advice.contains("reason=extra_quality_12b_wastes_shared_apple_memory"));
}

fn section_body<'a>(text: &'a str, section: &str) -> Option<&'a str> {
    text.lines().skip_while(|line| *line != section).nth(1)
}
