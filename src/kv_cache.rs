mod cache;
mod codec;
mod compaction;
mod feedback;
mod lookup;
mod model;
mod ops;
mod persistence;
mod retention;
mod store;

pub use cache::KvFusionCache;
#[cfg(test)]
use codec::legacy_backup_path;
pub use model::{
    MemoryCompactionMerge, MemoryCompactionPolicy, MemoryCompactionReport, MemoryEntry,
    MemoryMatch, MemoryRetentionPolicy, MemoryUpdateAction, MemoryUpdateReport, RetentionReport,
};

#[cfg(test)]
mod tests;
