use std::path::PathBuf;

use crate::Args;
use crate::model_service::json::{option_path_service_json, service_json_string};

pub(super) struct MatrixReportFiles {
    pub(super) trace: String,
    pub(super) memory: String,
    pub(super) experience: String,
    pub(super) adaptive: String,
    pub(super) response: String,
}

impl MatrixReportFiles {
    pub(super) fn from_args(args: &Args, response_path: Option<&PathBuf>) -> Self {
        Self {
            trace: option_path_service_json(args.trace_path.as_ref()),
            memory: service_json_string(&args.memory_path.display().to_string()),
            experience: service_json_string(&args.experience_path.display().to_string()),
            adaptive: service_json_string(&args.adaptive_path.display().to_string()),
            response: option_path_service_json(response_path),
        }
    }
}
