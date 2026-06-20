use super::RetrievalPreviewMatch;

pub(super) fn finite_number_text(value: &str, label: &str) -> Result<String, String> {
    parse_finite_number(value, label)?;
    Ok(value.to_owned())
}

pub(super) fn validate_empty_match_score_contract(
    total_matches: Option<usize>,
    max_score: Option<&str>,
) -> Result<(), String> {
    if total_matches == Some(0) && max_score.is_some() {
        return Err("experience retrieval preview empty match set forbids max_score".to_owned());
    }
    Ok(())
}

pub(super) fn validate_max_score(
    max_score: Option<&str>,
    matches: &[RetrievalPreviewMatch],
) -> Result<(), String> {
    let Some(max_score) = max_score else {
        return Ok(());
    };
    let max_score = parse_finite_number(max_score, "max_score=")?;
    for item in matches {
        let Some(score) = item.score.as_deref() else {
            continue;
        };
        let score = parse_finite_number(score, "match score")?;
        if score > max_score {
            return Err(format!(
                "experience retrieval preview max_score below rendered match score: max_score={max_score} match_id={} score={score}",
                item.id
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_score_order(matches: &[RetrievalPreviewMatch]) -> Result<(), String> {
    let mut previous = None;
    for item in matches {
        let Some(score) = item.score.as_deref() else {
            continue;
        };
        let score = parse_finite_number(score, "match score")?;
        if let Some((previous_id, previous_score)) = previous
            && score > previous_score
        {
            return Err(format!(
                "experience retrieval preview match score order drift: previous_match_id={previous_id} previous_score={previous_score} match_id={} score={score}",
                item.id
            ));
        }
        previous = Some((item.id.as_str(), score));
    }
    Ok(())
}

fn parse_finite_number(value: &str, label: &str) -> Result<f64, String> {
    let parsed = value.parse::<f64>().map_err(|_| {
        format!("experience retrieval preview expected finite number for {label}, got {value:?}")
    })?;
    if !parsed.is_finite() {
        return Err(format!(
            "experience retrieval preview expected finite number for {label}, got {value:?}"
        ));
    }
    Ok(parsed)
}
