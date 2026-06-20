#[derive(Debug, Clone)]
pub struct MemoryEntry {
    pub id: u64,
    pub key: String,
    pub vector: Vec<f32>,
    pub strength: f32,
    pub hits: u64,
    pub failures: u64,
    pub last_score: f32,
    pub created_at: u64,
    pub last_access: u64,
}

#[derive(Debug, Clone)]
pub struct MemoryMatch {
    pub id: u64,
    pub key: String,
    pub similarity: f32,
    pub strength: f32,
    pub vector: Vec<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemoryRetentionPolicy {
    pub stale_after: u64,
    pub decay_rate: f32,
    pub remove_below_strength: f32,
    pub remove_after_failures: u64,
}

impl Default for MemoryRetentionPolicy {
    fn default() -> Self {
        Self {
            stale_after: 64,
            decay_rate: 0.04,
            remove_below_strength: 0.04,
            remove_after_failures: 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryCompactionPolicy {
    pub similarity_threshold: f32,
    pub max_candidates: usize,
    pub max_merges: usize,
}

impl Default for MemoryCompactionPolicy {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.92,
            max_candidates: 512,
            max_merges: 32,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemoryCompactionMerge {
    pub primary_id: u64,
    pub removed_id: u64,
    pub similarity: f32,
}

#[derive(Debug, Clone)]
pub struct MemoryCompactionReport {
    pub before: usize,
    pub after: usize,
    pub merged: Vec<MemoryCompactionMerge>,
    pub removed: Vec<u64>,
}

impl MemoryCompactionReport {
    pub fn skipped(current_len: usize) -> Self {
        Self {
            before: current_len,
            after: current_len,
            merged: Vec::new(),
            removed: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RetentionReport {
    pub before: usize,
    pub after: usize,
    pub decayed: usize,
    pub removed: Vec<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryUpdateAction {
    Reinforce,
    Penalize,
}

impl MemoryUpdateAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Reinforce => "reinforce",
            Self::Penalize => "penalize",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemoryUpdateReport {
    pub id: u64,
    pub action: MemoryUpdateAction,
    pub requested_amount: f32,
    pub strength_before: Option<f32>,
    pub strength_after: Option<f32>,
    pub strength_delta: f32,
    pub removed: bool,
}

impl MemoryUpdateReport {
    pub fn missing(id: u64, action: MemoryUpdateAction, requested_amount: f32) -> Self {
        Self {
            id,
            action,
            requested_amount,
            strength_before: None,
            strength_after: None,
            strength_delta: 0.0,
            removed: false,
        }
    }

    pub fn applied(
        id: u64,
        action: MemoryUpdateAction,
        requested_amount: f32,
        strength_before: f32,
        strength_after: f32,
        removed: bool,
    ) -> Self {
        Self {
            id,
            action,
            requested_amount,
            strength_before: Some(strength_before),
            strength_after: Some(strength_after),
            strength_delta: strength_after - strength_before,
            removed,
        }
    }

    pub fn was_applied(self) -> bool {
        self.strength_before.is_some()
    }
}
