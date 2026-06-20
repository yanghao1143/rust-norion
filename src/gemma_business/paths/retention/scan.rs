use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use super::names::is_gemma_smoke_generated_run_name;

pub(super) fn gemma_smoke_generated_run_dirs(
    parent: &Path,
    base_name: &str,
) -> std::io::Result<Vec<PathBuf>> {
    let mut run_dirs = Vec::new();
    for entry in std::fs::read_dir(parent)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let file_name = entry.file_name();
        let Some(name) = file_name.to_str() else {
            continue;
        };
        if !is_gemma_smoke_generated_run_name(name, base_name) {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .unwrap_or(UNIX_EPOCH);
        run_dirs.push((modified, entry.path()));
    }

    run_dirs.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| right.1.cmp(&left.1)));
    Ok(run_dirs.into_iter().map(|(_, path)| path).collect())
}
