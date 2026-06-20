#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum RetrievalIndexContextActive {
    LatestTrustedDelimited,
}

pub(super) fn validate_index_context_contract(
    index_context_used: Option<bool>,
    index_context_chars: Option<usize>,
    index_context_query_chars: Option<usize>,
    index_context_query_trusted: Option<bool>,
    index_context_query_active_trusted: Option<bool>,
    index_context_query_context_active: Option<RetrievalIndexContextActive>,
) -> Result<(), String> {
    if index_context_used == Some(true) && index_context_chars.unwrap_or(0) == 0 {
        return Err(
            "experience retrieval preview index_context_used=true requires index_context_chars"
                .to_owned(),
        );
    }
    if index_context_used == Some(false) && index_context_chars.unwrap_or(0) > 0 {
        return Err(
            "experience retrieval preview index_context_used=false forbids index_context_chars"
                .to_owned(),
        );
    }
    if index_context_query_chars == Some(0) {
        return Err(
            "experience retrieval preview index_context_query chars must be positive".to_owned(),
        );
    }
    if index_context_query_chars.is_none() {
        return Ok(());
    }
    if index_context_used == Some(false) {
        return Err(
            "experience retrieval preview index_context_query requires index_context_used not false"
                .to_owned(),
        );
    }
    if let (Some(index_context_chars), Some(index_context_query_chars)) =
        (index_context_chars, index_context_query_chars)
        && index_context_chars != index_context_query_chars
    {
        return Err(format!(
            "experience retrieval preview index context chars mismatch: index_context_chars={index_context_chars} index_context_query_chars={index_context_query_chars}"
        ));
    }
    if index_context_query_trusted != Some(true) {
        return Err(
            "experience retrieval preview index_context_query requires trusted=true".to_owned(),
        );
    }
    if index_context_query_active_trusted.is_none() {
        return Err(
            "experience retrieval preview index_context_query missing active_trusted".to_owned(),
        );
    }
    if index_context_query_context_active
        != Some(RetrievalIndexContextActive::LatestTrustedDelimited)
    {
        return Err(
            "experience retrieval preview index_context_query requires context_active=latest_trusted_delimited"
                .to_owned(),
        );
    }
    Ok(())
}
