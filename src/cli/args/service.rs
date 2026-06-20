use super::values::parse_usize;
use std::path::PathBuf;

pub(crate) struct ServiceFlagParse<'a> {
    pub(crate) serve: &'a mut bool,
    pub(crate) serve_bind: &'a mut String,
    pub(crate) serve_max_requests: &'a mut Option<usize>,
    pub(crate) model_pool_manifest_path: &'a mut Option<PathBuf>,
}

impl ServiceFlagParse<'_> {
    pub(crate) fn parse(&mut self, raw: &[String], index: usize) -> Option<usize> {
        match raw.get(index)?.as_str() {
            "--serve" | "--model-service" => {
                *self.serve = true;
                Some(1)
            }
            "--serve-bind" | "--model-service-bind" => {
                let bind = raw.get(index + 1)?;
                *self.serve = true;
                *self.serve_bind = bind.clone();
                Some(2)
            }
            "--serve-max-requests" | "--model-service-max-requests" => {
                let max_requests = raw.get(index + 1)?;
                *self.serve = true;
                *self.serve_max_requests = Some(parse_usize(max_requests, 1).max(1));
                Some(2)
            }
            "--model-pool-manifest" | "--model-service-pool-manifest" => {
                let path = raw.get(index + 1)?;
                *self.serve = true;
                *self.model_pool_manifest_path = Some(PathBuf::from(path));
                Some(2)
            }
            _ => None,
        }
    }
}
