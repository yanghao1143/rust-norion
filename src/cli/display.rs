use std::path::PathBuf;

use rust_norion::{GistLevel, GistRecord, TierMigration, TierMigrationAction};

pub(crate) fn option_path_display(path: Option<&PathBuf>) -> String {
    path.map(|path| path.display().to_string())
        .unwrap_or_else(|| "none".to_owned())
}

pub(crate) fn option_u64_display(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_owned())
}

pub(crate) fn option_bool_display(value: Option<bool>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_owned())
}

pub(crate) fn count_gists(records: &[GistRecord], level: GistLevel) -> usize {
    records
        .iter()
        .filter(|record| record.level == level)
        .count()
}

pub(crate) fn count_tier_migrations(
    migrations: &[TierMigration],
    action: TierMigrationAction,
) -> usize {
    migrations
        .iter()
        .filter(|migration| migration.action == action)
        .count()
}
