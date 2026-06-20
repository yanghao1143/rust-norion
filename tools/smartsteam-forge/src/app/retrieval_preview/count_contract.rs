const MAX_RENDERED_RETRIEVAL_MATCHES: usize = 5;

pub(super) fn validate_count_evidence_present(
    match_count: Option<usize>,
    declared_matches: Option<usize>,
) -> Result<(), String> {
    if match_count.is_none() {
        return Err("experience retrieval preview missing match_count=".to_owned());
    }
    if declared_matches.is_none() {
        return Err("experience retrieval preview missing matches=".to_owned());
    }
    Ok(())
}

pub(super) fn validate_total_records_evidence_present(
    total_records: Option<usize>,
) -> Result<(), String> {
    if total_records.is_none() {
        return Err("experience retrieval preview missing total_records=".to_owned());
    }
    Ok(())
}

pub(super) fn validate_declared_matches(
    declared_matches: Option<usize>,
    rendered_matches: usize,
) -> Result<(), String> {
    let Some(declared_matches) = declared_matches else {
        return Ok(());
    };
    let expected_rendered = declared_matches.min(MAX_RENDERED_RETRIEVAL_MATCHES);
    if expected_rendered != rendered_matches {
        return Err(format!(
            "experience retrieval preview matches count mismatch: matches={declared_matches} rendered_match_lines={rendered_matches}"
        ));
    }
    Ok(())
}

pub(super) fn validate_match_count(
    match_count: Option<usize>,
    declared_matches: Option<usize>,
    rendered_matches: usize,
) -> Result<(), String> {
    let Some(match_count) = match_count else {
        return Ok(());
    };
    if let Some(declared_matches) = declared_matches
        && match_count != declared_matches
    {
        return Err(format!(
            "experience retrieval preview match_count drift: match_count={match_count} matches={declared_matches}"
        ));
    }
    let expected_rendered = match_count.min(MAX_RENDERED_RETRIEVAL_MATCHES);
    if expected_rendered != rendered_matches {
        return Err(format!(
            "experience retrieval preview match_count mismatch: match_count={match_count} rendered_match_lines={rendered_matches}"
        ));
    }
    Ok(())
}

pub(super) fn validate_retrieval_cardinality(
    total_records: Option<usize>,
    requested_limit: usize,
    total_matches: Option<usize>,
) -> Result<(), String> {
    let Some(total_matches) = total_matches else {
        return Ok(());
    };
    if total_matches > requested_limit {
        return Err(format!(
            "experience retrieval preview match count exceeds requested_limit: matches={total_matches} requested_limit={requested_limit}"
        ));
    }
    if let Some(total_records) = total_records
        && total_matches > total_records
    {
        return Err(format!(
            "experience retrieval preview match count exceeds total_records: matches={total_matches} total_records={total_records}"
        ));
    }
    Ok(())
}
