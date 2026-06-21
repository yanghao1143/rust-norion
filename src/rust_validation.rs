mod check;
mod output;
mod repair_loop;
mod report;
mod validator;

pub use check::RustSnippetCheck;
pub use repair_loop::{
    RustCodingCommandEvidence, RustCodingRepairCandidateSummary, RustCodingRepairCommandKind,
    RustCodingRepairDecision, RustCodingRepairFailureClass, RustCodingRepairHarness,
    RustCodingRepairInput, RustCodingRepairOutcome, RustCodingRepairPolicy, RustCodingRepairReport,
};
pub use report::RustSnippetCheckReport;
pub use validator::RustSnippetValidator;

#[cfg(test)]
mod tests;
