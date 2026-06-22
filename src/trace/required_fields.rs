mod auto_replay;
mod core;
mod evolution_ledger;
mod genome;
mod live_evolution;
mod memory;
mod retention;
mod routing;
mod specialized;

#[derive(Debug, Clone, Copy)]
pub(super) struct TraceRequiredField {
    pub(super) name: &'static str,
    pub(super) marker: &'static str,
}

pub(super) const fn required_field(name: &'static str, marker: &'static str) -> TraceRequiredField {
    TraceRequiredField { name, marker }
}

pub(super) fn trace_required_fields() -> impl Iterator<Item = &'static TraceRequiredField> {
    core::CORE_TRACE_REQUIRED_FIELDS
        .iter()
        .chain(memory::MEMORY_TRACE_REQUIRED_FIELDS)
        .chain(routing::ROUTING_TRACE_REQUIRED_FIELDS)
        .chain(auto_replay::AUTO_REPLAY_TRACE_REQUIRED_FIELDS)
        .chain(genome::GENOME_TRACE_REQUIRED_FIELDS)
        .chain(live_evolution::LIVE_EVOLUTION_TRACE_REQUIRED_FIELDS)
        .chain(evolution_ledger::EVOLUTION_LEDGER_TRACE_REQUIRED_FIELDS)
        .chain(retention::RETENTION_TRACE_REQUIRED_FIELDS)
}

pub(super) use specialized::{
    BUSINESS_CONTRACT_TRACE_REQUIRED_FIELDS, RUST_CHECK_TRACE_REQUIRED_FIELDS,
};
