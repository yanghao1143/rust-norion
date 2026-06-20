pub(super) fn validate_quality_metrics(
    total_records: Option<usize>,
    skipped_cross_task_pollution: Option<usize>,
    retrieval_noise_penalized_candidates: Option<usize>,
    retrieval_noise_filtered_candidates: Option<usize>,
    suppressed_prompt_index_candidates: Option<usize>,
    max_retrieval_noise_penalty: Option<&str>,
) -> Result<(), String> {
    validate_counter_within_total(
        "skipped_cross_task_pollution",
        skipped_cross_task_pollution,
        total_records,
    )?;
    validate_counter_within_total(
        "retrieval_noise_penalized_candidates",
        retrieval_noise_penalized_candidates,
        total_records,
    )?;
    validate_counter_within_total(
        "retrieval_noise_filtered_candidates",
        retrieval_noise_filtered_candidates,
        total_records,
    )?;
    validate_counter_within_total(
        "suppressed_prompt_index_candidates",
        suppressed_prompt_index_candidates,
        total_records,
    )?;
    if let (Some(filtered), Some(penalized)) = (
        retrieval_noise_filtered_candidates,
        retrieval_noise_penalized_candidates,
    ) && filtered > penalized
    {
        return Err(format!(
            "experience retrieval preview retrieval_noise_filtered_candidates exceeds retrieval_noise_penalized_candidates: filtered={filtered} penalized={penalized}"
        ));
    }

    let Some(max_retrieval_noise_penalty) = max_retrieval_noise_penalty else {
        return Ok(());
    };
    let parsed = max_retrieval_noise_penalty.parse::<f64>().map_err(|_| {
        format!(
            "experience retrieval preview expected finite number for max_retrieval_noise_penalty=, got {max_retrieval_noise_penalty:?}"
        )
    })?;
    if parsed < 0.0 {
        return Err(format!(
            "experience retrieval preview max_retrieval_noise_penalty must be non-negative: {parsed}"
        ));
    }
    if parsed > 0.0 && retrieval_noise_penalized_candidates == Some(0) {
        return Err(format!(
            "experience retrieval preview max_retrieval_noise_penalty requires retrieval_noise_penalized_candidates > 0: max_retrieval_noise_penalty={parsed}"
        ));
    }
    if parsed == 0.0 && retrieval_noise_penalized_candidates.is_some_and(|count| count > 0) {
        return Err(
            "experience retrieval preview max_retrieval_noise_penalty must be positive when retrieval_noise_penalized_candidates is positive"
                .to_owned(),
        );
    }
    Ok(())
}

fn validate_counter_within_total(
    name: &str,
    value: Option<usize>,
    total_records: Option<usize>,
) -> Result<(), String> {
    let (Some(value), Some(total_records)) = (value, total_records) else {
        return Ok(());
    };
    if value > total_records {
        return Err(format!(
            "experience retrieval preview {name} exceeds total_records: {name}={value} total_records={total_records}"
        ));
    }
    Ok(())
}
