use super::RetrievalPreviewMatch;
use super::index_context_contract::RetrievalIndexContextActive;
use super::score_contract::finite_number_text;

pub(super) fn retrieval_match_summary(line: &str) -> Result<RetrievalPreviewMatch, String> {
    let id = token_value(line, "id=")
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "experience retrieval preview match missing id".to_owned())?
        .to_owned();
    Ok(RetrievalPreviewMatch {
        id,
        score: optional_finite_number_token(line, "score=", "match score")?,
        runtime_model: token_value(line, "runtime_model=").map(str::to_owned),
        runtime_adapter: token_value(line, "runtime_adapter=").map(str::to_owned),
        runtime_device: token_value(line, "runtime_device=").map(str::to_owned),
        runtime_primary_lane: token_value(line, "runtime_primary_lane=").map(str::to_owned),
        runtime_fallback_lane: token_value(line, "runtime_fallback_lane=").map(str::to_owned),
        runtime_memory_mode: token_value(line, "runtime_memory_mode=").map(str::to_owned),
        runtime_device_execution_source: token_value(line, "runtime_device_execution_source=")
            .map(str::to_owned),
        runtime_forward_energy: optional_finite_number_token(
            line,
            "runtime_forward_energy=",
            "runtime_forward_energy",
        )?,
        runtime_kv_influence: optional_finite_number_token(
            line,
            "runtime_kv_influence=",
            "runtime_kv_influence",
        )?,
        runtime_uncertainty_perplexity: optional_finite_number_token(
            line,
            "runtime_uncertainty_perplexity=",
            "runtime_uncertainty_perplexity",
        )?,
        recursive_runtime_calls: optional_usize_token(line, "recursive_runtime_calls=")?,
        stored_runtime_kv_memory_ids: optional_u64_csv_token(
            line,
            "stored_runtime_kv_memory_ids=",
        )?
        .unwrap_or_default(),
    })
}

pub(super) fn require_line(lines: &[&str], index: usize, expected: &str) -> Result<(), String> {
    match lines.get(index) {
        Some(line) if *line == expected => Ok(()),
        Some(line) => Err(format!(
            "experience retrieval preview line {index} expected {expected:?}, got {line:?}"
        )),
        None => Err(format!(
            "experience retrieval preview missing line {index} expected {expected:?}"
        )),
    }
}

pub(super) fn required_usize_line(lines: &[&str], prefix: &str) -> Result<usize, String> {
    optional_usize_line(lines, prefix)?
        .ok_or_else(|| format!("experience retrieval preview missing {prefix}"))
}

pub(super) fn optional_usize_line(lines: &[&str], prefix: &str) -> Result<Option<usize>, String> {
    optional_line_value(lines, prefix)
        .map(|value| {
            value.parse::<usize>().map_err(|_| {
                format!("experience retrieval preview expected usize for {prefix}, got {value:?}")
            })
        })
        .transpose()
}

pub(super) fn optional_finite_number_line(
    lines: &[&str],
    prefix: &str,
) -> Result<Option<String>, String> {
    optional_line_value(lines, prefix)
        .map(|value| finite_number_text(value, prefix))
        .transpose()
}

pub(super) fn optional_bool_line(lines: &[&str], prefix: &str) -> Result<Option<bool>, String> {
    optional_line_value(lines, prefix)
        .map(|value| match value {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(format!(
                "experience retrieval preview expected bool for {prefix}, got {value:?}"
            )),
        })
        .transpose()
}

pub(super) fn optional_declared_matches(lines: &[&str]) -> Result<Option<usize>, String> {
    let Some(value) = optional_line_value(lines, "matches=") else {
        return Ok(None);
    };
    if value == "none" {
        return Ok(Some(0));
    }
    value.parse::<usize>().map(Some).map_err(|_| {
        format!("experience retrieval preview expected usize or none for matches=, got {value:?}")
    })
}

pub(super) fn optional_index_context_query_chars(lines: &[&str]) -> Result<Option<usize>, String> {
    let Some(line) = index_context_query_line(lines) else {
        return Ok(None);
    };
    let chars = token_value(line, "chars=").ok_or_else(|| {
        "experience retrieval preview index_context_query missing chars".to_owned()
    })?;
    chars.parse::<usize>().map(Some).map_err(|_| {
        format!("experience retrieval preview expected usize for index_context_query chars, got {chars:?}")
    })
}

pub(super) fn optional_index_context_query_trusted(lines: &[&str]) -> Result<Option<bool>, String> {
    let Some(line) = index_context_query_line(lines) else {
        return Ok(None);
    };
    let trusted = token_value(line, "trusted=").ok_or_else(|| {
        "experience retrieval preview index_context_query missing trusted".to_owned()
    })?;
    parse_bool_token(trusted, "index_context_query trusted")
}

pub(super) fn optional_index_context_query_active_trusted(
    lines: &[&str],
) -> Result<Option<bool>, String> {
    let Some(line) = index_context_query_line(lines) else {
        return Ok(None);
    };
    let active_trusted = token_value(line, "active_trusted=").ok_or_else(|| {
        "experience retrieval preview index_context_query missing active_trusted".to_owned()
    })?;
    parse_bool_token(active_trusted, "index_context_query active_trusted")
}

pub(super) fn optional_index_context_query_context_active(
    lines: &[&str],
) -> Result<Option<RetrievalIndexContextActive>, String> {
    let Some(line) = index_context_query_line(lines) else {
        return Ok(None);
    };
    match token_value(line, "context_active=").ok_or_else(|| {
        "experience retrieval preview index_context_query missing context_active".to_owned()
    })? {
        "latest_trusted_delimited" => Ok(Some(RetrievalIndexContextActive::LatestTrustedDelimited)),
        value => Err(format!(
            "experience retrieval preview unknown index_context_query context_active value {value:?}"
        )),
    }
}

fn optional_finite_number_token(
    line: &str,
    prefix: &str,
    label: &str,
) -> Result<Option<String>, String> {
    token_value(line, prefix)
        .map(|value| finite_number_text(value, label))
        .transpose()
}

fn optional_usize_token(line: &str, prefix: &str) -> Result<Option<usize>, String> {
    token_value(line, prefix)
        .map(|value| {
            value.parse::<usize>().map_err(|_| {
                format!("experience retrieval preview expected usize for {prefix}, got {value:?}")
            })
        })
        .transpose()
}

fn optional_u64_csv_token(line: &str, prefix: &str) -> Result<Option<Vec<u64>>, String> {
    token_value(line, prefix)
        .map(|value| {
            if value == "none" {
                return Ok(Vec::new());
            }
            value
                .split(',')
                .map(|part| {
                    part.parse::<u64>().map_err(|_| {
                        format!(
                            "experience retrieval preview expected u64 csv for {prefix}, got {value:?}"
                        )
                    })
                })
                .collect()
        })
        .transpose()
}

pub(super) fn optional_line_value<'a>(lines: &'a [&str], prefix: &str) -> Option<&'a str> {
    lines
        .iter()
        .find_map(|line| line.strip_prefix(prefix))
        .filter(|value| !value.trim().is_empty())
}

fn index_context_query_line<'a>(lines: &'a [&str]) -> Option<&'a str> {
    let Some(line) = lines
        .iter()
        .find(|line| line.starts_with("index_context_query="))
    else {
        return None;
    };
    Some(line)
}

fn token_value<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    line.split_whitespace()
        .find_map(|token| token.strip_prefix(prefix))
        .filter(|value| !value.trim().is_empty())
}

fn parse_bool_token(value: &str, label: &str) -> Result<Option<bool>, String> {
    match value {
        "true" => Ok(Some(true)),
        "false" => Ok(Some(false)),
        _ => Err(format!(
            "experience retrieval preview expected bool for {label}, got {value:?}"
        )),
    }
}
