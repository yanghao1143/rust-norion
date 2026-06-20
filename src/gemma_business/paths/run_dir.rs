use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::Args;
use crate::gemma_business::{
    GEMMA_BUSINESS_CYCLE_SMOKE_DIR, GEMMA_BUSINESS_SMOKE_DIR, GEMMA_MODEL_SERVICE_SMOKE_DIR,
};

pub(crate) fn gemma_smoke_run_dir(base_dir: &str) -> PathBuf {
    let run_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    PathBuf::from(format!("{base_dir}-{run_id}"))
}

pub(crate) fn gemma_smoke_base_dir(args: &Args) -> Option<&'static str> {
    if args.gemma_model_service_smoke {
        Some(GEMMA_MODEL_SERVICE_SMOKE_DIR)
    } else if args.gemma_business_cycle_smoke {
        Some(GEMMA_BUSINESS_CYCLE_SMOKE_DIR)
    } else if args.gemma_business_smoke {
        Some(GEMMA_BUSINESS_SMOKE_DIR)
    } else {
        None
    }
}
