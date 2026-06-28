#[cfg(test)]
pub(crate) use crate::adaptive_state::EvolutionLedger;
#[cfg(test)]
pub(crate) use crate::hardware::DeviceClass;
#[cfg(test)]
pub(crate) use crate::kv_cache::{MemoryCompactionPolicy, MemoryRetentionPolicy};

const STATE_INSPECTION_FLOAT_EPSILON: f32 = 0.000_001;

mod build;
mod evaluate;
mod evidence;
mod gate;
mod memory;
mod report;
mod runtime_evidence;
mod stats;
mod summary_line;
mod thresholds;

use evidence::*;
pub use gate::*;
use memory::{
    format_memory_vector_dimensions, memory_vector_dimensions, runtime_kv_vector_dimensions,
    top_memory_summaries,
};
pub use report::{
    StateExperienceHygieneFinding, StateExperienceIndexFinding, StateExperienceSummary,
    StateInspectionReport, StateMemorySummary, StateMemoryVectorDimensions,
};
use runtime_evidence::{
    has_runtime_architecture_evidence, has_text, inspection_hardware_plan,
    runtime_adapter_selection_mismatch_count, runtime_kv_precision_mismatch_count,
};
pub use stats::{
    BusinessContractInspectionStats, ExternalSemanticContextInspectionStats,
    FhtDkeBudgetInspectionStats, RuntimeErrorInspectionStats, RustCheckInspectionStats,
    SelfEvolvingMemoryWritebackInspectionStats,
};
use thresholds::{
    require_max_f32, require_max_u64, require_max_usize, require_min_f32, require_min_u64,
    require_min_usize,
};

#[cfg(test)]
mod tests;
