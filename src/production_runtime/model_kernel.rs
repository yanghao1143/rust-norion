use std::sync::{Arc, Mutex};

use crate::runtime::{ModelRuntime, RuntimeError};

use super::kernel::{ProductionForwardKernel, ProductionKernelContext, ProductionKernelOutput};

#[derive(Debug, Clone)]
pub struct ModelRuntimeForwardKernel<R> {
    runtime: Arc<Mutex<R>>,
}

impl<R> ModelRuntimeForwardKernel<R> {
    pub fn new(runtime: R) -> Self {
        Self {
            runtime: Arc::new(Mutex::new(runtime)),
        }
    }

    pub fn with_shared_runtime(runtime: Arc<Mutex<R>>) -> Self {
        Self { runtime }
    }

    pub fn runtime(&self) -> Arc<Mutex<R>> {
        Arc::clone(&self.runtime)
    }
}

impl<R> ProductionForwardKernel for ModelRuntimeForwardKernel<R>
where
    R: ModelRuntime + std::fmt::Debug + Send + 'static,
{
    fn generate(
        &self,
        context: ProductionKernelContext<'_>,
    ) -> Result<ProductionKernelOutput, RuntimeError> {
        let mut runtime = self
            .runtime
            .lock()
            .map_err(|_| RuntimeError::new("model runtime forward kernel lock is poisoned"))?;
        let imported = runtime.import_kv(context.imported_kv_blocks)?;
        let mut request = context.request.clone();
        request.runtime_metadata = context.manifest.runtime_metadata();
        request.runtime_architecture = context.manifest.architecture;
        request.imported_kv_blocks = context.imported_kv_blocks.to_vec();

        let response = runtime.generate(request)?;
        let exported_kv_blocks = runtime.export_kv()?;
        let mut diagnostics = response.diagnostics;
        diagnostics.imported_kv_blocks = imported;
        diagnostics.exported_kv_blocks = exported_kv_blocks.len();
        if imported > 0 && !diagnostics.has_runtime_kv_segment_signal() {
            diagnostics.runtime_kv_segments_included = imported;
        }

        Ok(ProductionKernelOutput::new(response.answer)
            .with_tokens(response.tokens)
            .with_trace(response.trace)
            .with_diagnostics(diagnostics)
            .with_exported_kv_blocks(exported_kv_blocks))
    }
}
