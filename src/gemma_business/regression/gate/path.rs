use std::path::{Path, PathBuf};

use crate::gemma_business::GEMMA_BUSINESS_CYCLE_SMOKE_REPORT_FILE;

pub fn gemma_business_regression_report_path(path: &Path) -> PathBuf {
    if path.is_dir() {
        path.join(GEMMA_BUSINESS_CYCLE_SMOKE_REPORT_FILE)
    } else {
        path.to_path_buf()
    }
}
