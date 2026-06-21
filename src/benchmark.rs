use crate::hardware::DeviceClass;
#[cfg(test)]
use crate::{adaptive_state::EvolutionLedger, drift::DriftSeverity, hierarchy::TaskProfile};
mod cases;
mod display;
mod embedding_evidence;
mod gate;
mod genome_evidence;
mod improvement_corpus_evidence;
mod kv_quant;
mod live_evidence;
mod memory_evidence;
mod reflection_evidence;
mod roundtrip;
mod routing_evidence;
mod runtime_evidence;
mod self_evolving_memory_evidence;
mod summary;
mod summary_gate;

#[cfg(test)]
use cases::long_context_benchmark_prompt;
pub use cases::{BenchmarkCase, default_benchmark_cases};
pub use embedding_evidence::BenchmarkEmbeddingEvidence;
pub use gate::{BenchmarkGate, BenchmarkGateReport};
pub use genome_evidence::BenchmarkGenomeEvidence;
pub use improvement_corpus_evidence::BenchmarkImprovementCorpusEvidence;
pub use kv_quant::{
    KvQuantBenchmarkCaseResult, KvQuantBenchmarkGate, KvQuantBenchmarkGateReport,
    KvQuantBenchmarkSummary,
};
pub use live_evidence::BenchmarkLiveEvolutionEvidence;
pub use memory_evidence::BenchmarkMemoryGovernanceEvidence;
pub use reflection_evidence::BenchmarkReflectionEvidence;
pub use roundtrip::{
    PersistentRoundtripDeviceReport, PersistentRoundtripInput, PersistentRoundtripMatrixReport,
    PersistentRoundtripReport,
};
pub use routing_evidence::BenchmarkRoutingEvidence;
pub use runtime_evidence::{
    BenchmarkRuntimeArchitectureEvidence, BenchmarkRuntimeDeviceExecutionEvidence,
};
pub use self_evolving_memory_evidence::{
    SelfEvolvingMemoryAbCase, SelfEvolvingMemoryAbGate, SelfEvolvingMemoryAbGateReport,
    SelfEvolvingMemoryAbHarness, SelfEvolvingMemoryAbRecommendation, SelfEvolvingMemoryAbReport,
    SelfEvolvingMemoryAbResult, SelfEvolvingMemoryEvalLanguage, SelfEvolvingMemoryEvalMode,
    SelfEvolvingMemoryValidationEvidence, default_self_evolving_memory_ab_cases,
    run_default_self_evolving_memory_ab_suite, seeded_self_evolving_memory_ab_store,
};
pub use summary::{BenchmarkCaseResult, BenchmarkSummary};

fn push_unique_device(devices: &mut Vec<DeviceClass>, device: DeviceClass) {
    if device != DeviceClass::Auto && !devices.contains(&device) {
        devices.push(device);
    }
}

fn devices_csv(devices: Vec<DeviceClass>) -> String {
    let devices = devices
        .into_iter()
        .map(DeviceClass::as_str)
        .collect::<Vec<_>>();

    if devices.is_empty() {
        "none".to_owned()
    } else {
        devices.join("+")
    }
}

fn explicit_device_count(devices: &[DeviceClass]) -> usize {
    DeviceClass::explicit_profiles()
        .iter()
        .filter(|device| devices.contains(device))
        .count()
}

#[cfg(test)]
mod tests;
