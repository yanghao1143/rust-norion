use std::fs;
use std::path::Path;

const CONFIG_FILE: &str = "config.json";
const TOKENIZER_ASSETS: &[&str] = &["tokenizer.json", "tokenizer.model", "tokenizer_config.json"];
const WEIGHT_INDEX_FILE: &str = "model.safetensors.index.json";
const WEIGHT_EXTENSIONS: &[&str] = &[".safetensors", ".bin"];

pub(super) fn has_gemma_config_asset(model_path: &Path) -> bool {
    model_path.join(CONFIG_FILE).is_file()
}

pub(super) fn has_gemma_tokenizer_asset(model_path: &Path) -> bool {
    has_any_file(model_path, TOKENIZER_ASSETS)
}

pub(super) fn has_gemma_weight_asset(model_path: &Path) -> bool {
    let Ok(entries) = fs::read_dir(model_path) else {
        return false;
    };
    entries.filter_map(Result::ok).any(|entry| {
        entry
            .file_name()
            .to_str()
            .map(is_gemma_weight_asset_name)
            .unwrap_or(false)
            && entry.path().is_file()
    })
}

fn has_any_file(model_path: &Path, names: &[&str]) -> bool {
    names.iter().any(|name| model_path.join(name).is_file())
}

fn is_gemma_weight_asset_name(name: &str) -> bool {
    WEIGHT_EXTENSIONS
        .iter()
        .any(|extension| name.ends_with(extension))
        || name == WEIGHT_INDEX_FILE
}

#[cfg(test)]
mod tests {
    use super::is_gemma_weight_asset_name;

    #[test]
    fn gemma_weight_asset_name_accepts_index_and_weight_extensions() {
        assert!(is_gemma_weight_asset_name("model.safetensors.index.json"));
        assert!(is_gemma_weight_asset_name(
            "model-00001-of-00004.safetensors"
        ));
        assert!(is_gemma_weight_asset_name("pytorch_model.bin"));
        assert!(!is_gemma_weight_asset_name("tokenizer.json"));
    }
}
