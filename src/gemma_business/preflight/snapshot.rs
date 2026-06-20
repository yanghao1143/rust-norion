use std::path::Path;

mod assets;
mod checks;

use assets::{has_gemma_config_asset, has_gemma_tokenizer_asset, has_gemma_weight_asset};
use checks::require_snapshot_asset;

pub(super) fn gemma_local_snapshot_asset_failures(model_path: &Path) -> Vec<String> {
    let mut failures = Vec::new();
    if !model_path.is_dir() {
        failures.push(format!(
            "Gemma business smoke local snapshot must be a directory: {}",
            model_path.display()
        ));
        return failures;
    }

    require_snapshot_asset(
        has_gemma_config_asset(model_path),
        model_path,
        "missing config.json",
        &mut failures,
    );
    require_snapshot_asset(
        has_gemma_tokenizer_asset(model_path),
        model_path,
        "missing tokenizer asset",
        &mut failures,
    );
    require_snapshot_asset(
        has_gemma_weight_asset(model_path),
        model_path,
        "missing weight asset (*.safetensors or *.bin)",
        &mut failures,
    );
    failures
}
