#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfiniMemoryScope {
    LocalWindow,
    GlobalMemory,
    Skipped,
}

#[derive(Debug, Clone)]
pub struct InfiniMemoryItem {
    pub id: u64,
    pub key: String,
    pub vector: Vec<f32>,
    pub scope: InfiniMemoryScope,
    pub score: f32,
    pub estimated_tokens: usize,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct InfiniMemoryCounts {
    pub local_window: usize,
    pub global_memory: usize,
    pub skipped: usize,
    pub local_tokens: usize,
    pub global_tokens: usize,
    pub skipped_tokens: usize,
}

#[derive(Debug, Clone, Default)]
pub struct InfiniMemoryPlan {
    local_window: Vec<InfiniMemoryItem>,
    global_memory: Vec<InfiniMemoryItem>,
    skipped: Vec<InfiniMemoryItem>,
}

impl InfiniMemoryPlan {
    pub fn new(
        local_window: Vec<InfiniMemoryItem>,
        global_memory: Vec<InfiniMemoryItem>,
        skipped: Vec<InfiniMemoryItem>,
    ) -> Self {
        Self {
            local_window,
            global_memory,
            skipped,
        }
    }

    pub fn local_window(&self) -> &[InfiniMemoryItem] {
        &self.local_window
    }

    pub fn global_memory(&self) -> &[InfiniMemoryItem] {
        &self.global_memory
    }

    pub fn skipped(&self) -> &[InfiniMemoryItem] {
        &self.skipped
    }

    pub fn counts(&self) -> InfiniMemoryCounts {
        InfiniMemoryCounts {
            local_window: self.local_window.len(),
            global_memory: self.global_memory.len(),
            skipped: self.skipped.len(),
            local_tokens: self
                .local_window
                .iter()
                .map(|item| item.estimated_tokens)
                .sum(),
            global_tokens: self
                .global_memory
                .iter()
                .map(|item| item.estimated_tokens)
                .sum(),
            skipped_tokens: self.skipped.iter().map(|item| item.estimated_tokens).sum(),
        }
    }
}
