use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeManifestValidation {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl RuntimeManifestValidation {
    pub fn passed(&self) -> bool {
        self.errors.is_empty()
    }
}

pub(super) fn validate_required_asset_file(
    label: &str,
    path: Option<&Path>,
    errors: &mut Vec<String>,
) {
    let Some(path) = path else {
        errors.push(format!(
            "{label} asset path is required for production runtimes"
        ));
        return;
    };

    validate_optional_asset_file(label, path, errors);
}

pub(super) fn validate_optional_asset_file(label: &str, path: &Path, errors: &mut Vec<String>) {
    if path.as_os_str().is_empty() {
        errors.push(format!("{label} asset path must not be empty"));
        return;
    }
    if !path.exists() {
        errors.push(format!(
            "{} asset path does not exist: {}",
            label,
            path.display()
        ));
        return;
    }
    if !path.is_file() {
        errors.push(format!(
            "{} asset path is not a file: {}",
            label,
            path.display()
        ));
    }
}
