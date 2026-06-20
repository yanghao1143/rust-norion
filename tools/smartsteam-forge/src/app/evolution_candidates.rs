use std::{io, io::Write, path::PathBuf};

use super::evolution_candidate_backlog::{append_candidate_backlog, read_candidate_backlog_items};
use super::evolution_candidate_lifecycle::{
    candidate_lifecycle_gate, normalize_candidate_list_status_filter, select_apply_check_candidate,
    suggested_candidate_scope, suggested_candidate_validation_command,
};
use super::evolution_candidate_model::{EvolutionCandidatePaths, candidate_backlog_path};
use super::evolution_candidate_render::{
    CandidateApplyCheckRender, render_candidate_apply_check_text, render_candidate_batch_text,
    render_candidate_gate_text, render_candidate_list_text, render_candidate_mark_text,
    render_candidate_validation_text, render_empty_candidates_text,
};
use super::evolution_candidate_sources::load_candidate_batch;
use super::evolution_candidate_updates::{mark_candidate_backlog, validate_candidate_backlog};

pub fn run_evolution_candidates(
    work_dir: &str,
    limit: usize,
    save_backlog: Option<&str>,
) -> io::Result<()> {
    let mut stdout = io::stdout();
    run_evolution_candidates_to(work_dir, limit, save_backlog, &mut stdout)
}

pub fn run_evolution_candidate_list(
    work_dir: &str,
    backlog_path: Option<&str>,
    status_filter: Option<&str>,
    limit: usize,
) -> io::Result<()> {
    let mut stdout = io::stdout();
    run_evolution_candidate_list_to(work_dir, backlog_path, status_filter, limit, &mut stdout)
}

pub fn run_evolution_candidate_gate(work_dir: &str, backlog_path: Option<&str>) -> io::Result<()> {
    let mut stdout = io::stdout();
    run_evolution_candidate_gate_to(work_dir, backlog_path, &mut stdout)
}

pub fn run_evolution_candidate_apply_check(
    work_dir: &str,
    backlog_path: Option<&str>,
    candidate_selector: &str,
) -> io::Result<()> {
    let mut stdout = io::stdout();
    run_evolution_candidate_apply_check_to(work_dir, backlog_path, candidate_selector, &mut stdout)
}

pub fn run_evolution_candidate_validate(
    work_dir: &str,
    backlog_path: Option<&str>,
    candidate_id: &str,
    command: &str,
    status_code: &str,
    note: Option<&str>,
) -> io::Result<()> {
    let mut stdout = io::stdout();
    run_evolution_candidate_validate_to(
        work_dir,
        backlog_path,
        candidate_id,
        command,
        status_code,
        note,
        &mut stdout,
    )
}

pub fn run_evolution_candidate_mark(
    work_dir: &str,
    backlog_path: Option<&str>,
    candidate_id: &str,
    status: &str,
    note: Option<&str>,
) -> io::Result<()> {
    let mut stdout = io::stdout();
    run_evolution_candidate_mark_to(
        work_dir,
        backlog_path,
        candidate_id,
        status,
        note,
        &mut stdout,
    )
}

fn run_evolution_candidate_mark_to<W: Write>(
    work_dir: &str,
    backlog_path: Option<&str>,
    candidate_id: &str,
    status: &str,
    note: Option<&str>,
    output: &mut W,
) -> io::Result<()> {
    writeln!(
        output,
        "{}",
        mark_evolution_candidate(work_dir, backlog_path, candidate_id, status, note)?
    )?;
    output.flush()
}

fn run_evolution_candidate_validate_to<W: Write>(
    work_dir: &str,
    backlog_path: Option<&str>,
    candidate_id: &str,
    command: &str,
    status_code: &str,
    note: Option<&str>,
    output: &mut W,
) -> io::Result<()> {
    writeln!(
        output,
        "{}",
        validate_evolution_candidate(
            work_dir,
            backlog_path,
            candidate_id,
            command,
            status_code,
            note,
        )?
    )?;
    output.flush()
}

fn run_evolution_candidate_list_to<W: Write>(
    work_dir: &str,
    backlog_path: Option<&str>,
    status_filter: Option<&str>,
    limit: usize,
    output: &mut W,
) -> io::Result<()> {
    writeln!(
        output,
        "{}",
        render_evolution_candidate_list(work_dir, backlog_path, status_filter, limit)?
    )?;
    output.flush()
}

fn run_evolution_candidate_gate_to<W: Write>(
    work_dir: &str,
    backlog_path: Option<&str>,
    output: &mut W,
) -> io::Result<()> {
    let (text, ready) = render_evolution_candidate_gate(work_dir, backlog_path)?;
    writeln!(output, "{text}")?;
    output.flush()?;
    if ready {
        Ok(())
    } else {
        Err(io::Error::other("candidate lifecycle gate failed"))
    }
}

fn render_evolution_candidate_gate(
    work_dir: &str,
    backlog_path: Option<&str>,
) -> io::Result<(String, bool)> {
    let paths = EvolutionCandidatePaths::new(work_dir);
    let backlog_path = candidate_backlog_path(&paths, backlog_path);
    let gate = candidate_lifecycle_gate(&backlog_path)?;
    let ready = gate.ready();
    Ok((render_candidate_gate_text(work_dir, &gate, ready), ready))
}

fn run_evolution_candidate_apply_check_to<W: Write>(
    work_dir: &str,
    backlog_path: Option<&str>,
    candidate_selector: &str,
    output: &mut W,
) -> io::Result<()> {
    writeln!(
        output,
        "{}",
        render_evolution_candidate_apply_check(work_dir, backlog_path, candidate_selector)?
    )?;
    output.flush()
}

fn render_evolution_candidate_apply_check(
    work_dir: &str,
    backlog_path: Option<&str>,
    candidate_selector: &str,
) -> io::Result<String> {
    let candidate_selector = candidate_selector.trim();
    if candidate_selector.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "candidate selector is required",
        ));
    }
    let paths = EvolutionCandidatePaths::new(work_dir);
    let backlog_path = candidate_backlog_path(&paths, backlog_path);
    let exists = backlog_path.is_file();
    let (items, invalid_count) = read_candidate_backlog_items(&backlog_path)?;
    let item = select_apply_check_candidate(&items, candidate_selector, &backlog_path)?;
    let apply_ready = item.status == "accepted";
    let status_gate = if apply_ready { "pass" } else { "blocked" };
    let block_reason = if apply_ready {
        "none"
    } else {
        "candidate_status_not_accepted"
    };
    let suggested_scope = suggested_candidate_scope(&item.answer_preview);
    let suggested_validation = suggested_candidate_validation_command(&item.answer_preview);

    Ok(render_candidate_apply_check_text(
        CandidateApplyCheckRender {
            work_dir,
            backlog_path: &backlog_path,
            exists,
            total: items.len(),
            invalid_count,
            candidate_selector,
            item: &item,
            apply_ready,
            status_gate,
            block_reason,
            suggested_scope,
            suggested_validation,
        },
    ))
}

fn render_evolution_candidate_list(
    work_dir: &str,
    backlog_path: Option<&str>,
    status_filter: Option<&str>,
    limit: usize,
) -> io::Result<String> {
    let limit = limit.max(1);
    let paths = EvolutionCandidatePaths::new(work_dir);
    let backlog_path = candidate_backlog_path(&paths, backlog_path);
    let filter = normalize_candidate_list_status_filter(status_filter)?;
    let exists = backlog_path.is_file();
    let (items, invalid_count) = read_candidate_backlog_items(&backlog_path)?;
    let matched = items
        .iter()
        .filter(|item| {
            filter
                .as_deref()
                .map_or(true, |status| item.status == status)
        })
        .take(limit)
        .collect::<Vec<_>>();
    Ok(render_candidate_list_text(
        work_dir,
        &backlog_path,
        exists,
        filter.as_deref().unwrap_or("all"),
        items.len(),
        &matched,
        invalid_count,
        limit,
    ))
}

fn validate_evolution_candidate(
    work_dir: &str,
    backlog_path: Option<&str>,
    candidate_id: &str,
    command: &str,
    status_code: &str,
    note: Option<&str>,
) -> io::Result<String> {
    let update = validate_candidate_backlog(
        work_dir,
        backlog_path,
        candidate_id,
        command,
        status_code,
        note,
    )?;

    Ok(render_candidate_validation_text(work_dir, &update))
}

fn mark_evolution_candidate(
    work_dir: &str,
    backlog_path: Option<&str>,
    candidate_id: &str,
    status: &str,
    note: Option<&str>,
) -> io::Result<String> {
    let update = mark_candidate_backlog(work_dir, backlog_path, candidate_id, status, note)?;

    Ok(render_candidate_mark_text(work_dir, &update))
}

fn run_evolution_candidates_to<W: Write>(
    work_dir: &str,
    limit: usize,
    save_backlog: Option<&str>,
    output: &mut W,
) -> io::Result<()> {
    writeln!(
        output,
        "{}",
        render_evolution_candidates(work_dir, limit, save_backlog)?
    )?;
    output.flush()
}

fn render_evolution_candidates(
    work_dir: &str,
    limit: usize,
    save_backlog: Option<&str>,
) -> io::Result<String> {
    let limit = limit.max(1);
    let paths = EvolutionCandidatePaths::new(work_dir);
    let batch = load_candidate_batch(&paths, limit)?;
    let backlog_path = save_backlog.map(|path| {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            paths.backlog()
        } else {
            PathBuf::from(trimmed)
        }
    });
    let backlog = match save_backlog {
        Some(_) => Some(append_candidate_backlog(
            backlog_path.as_deref().expect("candidate backlog path"),
            batch
                .as_ref()
                .map(|batch| batch.candidates.as_slice())
                .unwrap_or(&[]),
        )?),
        None => None,
    };

    if let Some(batch) = batch {
        return Ok(render_candidate_batch_text(
            work_dir,
            limit,
            &batch,
            backlog.as_ref(),
        ));
    }

    Ok(render_empty_candidates_text(
        work_dir,
        limit,
        &paths,
        backlog.as_ref(),
    ))
}

#[cfg(test)]
mod tests;
