use crate::kv_exchange::RuntimeKvBlock;

#[derive(Debug, Clone, Default)]
pub(super) struct LocalRuntimeSession {
    imported_kv_blocks: Vec<RuntimeKvBlock>,
    imported_kv_pending: bool,
    exported_kv_blocks: Vec<RuntimeKvBlock>,
}

impl LocalRuntimeSession {
    pub(super) fn imported_kv_blocks(&self) -> &[RuntimeKvBlock] {
        &self.imported_kv_blocks
    }

    pub(super) fn imported_kv_pending(&self) -> bool {
        self.imported_kv_pending
    }

    pub(super) fn exported_kv_blocks(&self) -> &[RuntimeKvBlock] {
        &self.exported_kv_blocks
    }

    pub(super) fn import_kv(&mut self, blocks: &[RuntimeKvBlock], max_blocks: usize) -> usize {
        self.set_imported_kv(blocks, max_blocks, true)
    }

    pub(super) fn set_request_imported_kv(
        &mut self,
        blocks: &[RuntimeKvBlock],
        max_blocks: usize,
    ) -> usize {
        self.set_imported_kv(blocks, max_blocks, false)
    }

    pub(super) fn consume_pending_imported_kv(&mut self) {
        self.imported_kv_pending = false;
    }

    pub(super) fn clear_imported_kv(&mut self) {
        self.imported_kv_blocks.clear();
        self.imported_kv_pending = false;
    }

    fn set_imported_kv(
        &mut self,
        blocks: &[RuntimeKvBlock],
        max_blocks: usize,
        pending: bool,
    ) -> usize {
        self.imported_kv_blocks = blocks.iter().take(max_blocks).cloned().collect();
        self.imported_kv_pending = pending && !self.imported_kv_blocks.is_empty();
        self.imported_kv_blocks.len()
    }

    pub(super) fn export_kv(&self, max_blocks: usize) -> Vec<RuntimeKvBlock> {
        self.exported_kv_blocks
            .iter()
            .take(max_blocks)
            .cloned()
            .collect()
    }

    pub(super) fn replace_exported_kv(&mut self, blocks: Vec<RuntimeKvBlock>) {
        self.exported_kv_blocks = blocks;
    }
}
