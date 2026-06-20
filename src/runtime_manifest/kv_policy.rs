#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeKvPolicy {
    pub import_enabled: bool,
    pub export_enabled: bool,
    pub max_import_blocks: usize,
    pub max_export_blocks: usize,
}

impl RuntimeKvPolicy {
    pub fn disabled() -> Self {
        Self {
            import_enabled: false,
            export_enabled: false,
            max_import_blocks: 0,
            max_export_blocks: 0,
        }
    }

    pub fn import_export() -> Self {
        Self {
            import_enabled: true,
            export_enabled: true,
            max_import_blocks: 8,
            max_export_blocks: 4,
        }
    }

    pub fn from_capabilities(import_enabled: bool, export_enabled: bool) -> Self {
        Self {
            import_enabled,
            export_enabled,
            max_import_blocks: if import_enabled { 8 } else { 0 },
            max_export_blocks: if export_enabled { 4 } else { 0 },
        }
    }
}
