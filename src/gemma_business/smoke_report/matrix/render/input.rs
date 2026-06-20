use crate::gemma_business::smoke_report::matrix::contract::MatrixReportContract;
use crate::gemma_business::smoke_report::matrix::evidence::MatrixReportEvidence;
use crate::gemma_business::smoke_report::matrix::files::MatrixReportFiles;
use crate::gemma_business::smoke_report::matrix::sections::MatrixReportSections;
use crate::gemma_business::smoke_report::matrix::summary::MatrixReportSummary;

pub(in crate::gemma_business::smoke_report::matrix) struct MatrixReportRender<'a> {
    pub(in crate::gemma_business::smoke_report::matrix) passed: bool,
    pub(in crate::gemma_business::smoke_report::matrix) bind: &'a str,
    pub(in crate::gemma_business::smoke_report::matrix) evidence: &'a MatrixReportEvidence,
    pub(in crate::gemma_business::smoke_report::matrix) summary: &'a MatrixReportSummary,
    pub(in crate::gemma_business::smoke_report::matrix) runtime_token_count: u64,
    pub(in crate::gemma_business::smoke_report::matrix) files: &'a MatrixReportFiles,
    pub(in crate::gemma_business::smoke_report::matrix) sections: &'a MatrixReportSections,
    pub(in crate::gemma_business::smoke_report::matrix) contract: &'a MatrixReportContract,
    pub(in crate::gemma_business::smoke_report::matrix) feedback_applied: u64,
    pub(in crate::gemma_business::smoke_report::matrix) rust_check_feedback_applied: u64,
    pub(in crate::gemma_business::smoke_report::matrix) checked_trace_lines: u64,
    pub(in crate::gemma_business::smoke_report::matrix) case_json: &'a str,
    pub(in crate::gemma_business::smoke_report::matrix) failures: &'a [String],
}
