use std::collections::BTreeSet;

use crate::hierarchy::TaskProfile;
use crate::privacy_redaction::contains_private_or_executable_marker;
use crate::self_evolving_memory::{
    SelfEvolvingEpisodeInput, SelfEvolvingHeuristicInput, SelfEvolvingMemoryApproval,
    SelfEvolvingMemoryQuery, SelfEvolvingMemoryRetrievalReport, SelfEvolvingMemoryStore,
    ToolReliabilityObservationInput,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SelfEvolvingMemoryEvalMode {
    Baseline,
    Episodic,
    Heuristic,
    ToolReliability,
    Combined,
}

impl SelfEvolvingMemoryEvalMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Baseline => "baseline",
            Self::Episodic => "episodic",
            Self::Heuristic => "heuristic",
            Self::ToolReliability => "tool_reliability",
            Self::Combined => "combined",
        }
    }

    fn includes_episode(self) -> bool {
        matches!(self, Self::Episodic | Self::Combined)
    }

    fn includes_heuristic(self) -> bool {
        matches!(self, Self::Heuristic | Self::Combined)
    }

    fn includes_tool_reliability(self) -> bool {
        matches!(self, Self::ToolReliability | Self::Combined)
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::Baseline,
            Self::Episodic,
            Self::Heuristic,
            Self::ToolReliability,
            Self::Combined,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SelfEvolvingMemoryEvalLanguage {
    English,
    Chinese,
    RustCoding,
}

impl SelfEvolvingMemoryEvalLanguage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::English => "english",
            Self::Chinese => "chinese",
            Self::RustCoding => "rust_coding",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfEvolvingMemoryAbRecommendation {
    Noop,
    HoldForEvidence,
    HoldForApproval,
    Quarantine,
    Rollback,
}

impl SelfEvolvingMemoryAbRecommendation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Noop => "noop",
            Self::HoldForEvidence => "hold_for_evidence",
            Self::HoldForApproval => "hold_for_approval",
            Self::Quarantine => "quarantine",
            Self::Rollback => "rollback",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelfEvolvingMemoryValidationEvidence {
    pub compiler_passed: bool,
    pub tests_passed: bool,
    pub benchmark_passed: bool,
}

impl SelfEvolvingMemoryValidationEvidence {
    pub fn passed() -> Self {
        Self {
            compiler_passed: true,
            tests_passed: true,
            benchmark_passed: true,
        }
    }

    pub fn failed() -> Self {
        Self {
            compiler_passed: false,
            tests_passed: false,
            benchmark_passed: false,
        }
    }

    pub fn all_passed(self) -> bool {
        self.compiler_passed && self.tests_passed && self.benchmark_passed
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelfEvolvingMemoryAbCase {
    pub id: String,
    pub language: SelfEvolvingMemoryEvalLanguage,
    pub profile: TaskProfile,
    pub prompt: String,
    pub tags: Vec<String>,
    pub baseline_quality: f32,
    pub baseline_latency_ms: u128,
    pub baseline_token_proxy: usize,
    pub validation: SelfEvolvingMemoryValidationEvidence,
}

impl SelfEvolvingMemoryAbCase {
    pub fn new(
        id: impl Into<String>,
        language: SelfEvolvingMemoryEvalLanguage,
        profile: TaskProfile,
        prompt: impl Into<String>,
        tags: Vec<String>,
        baseline_quality: f32,
        baseline_token_proxy: usize,
    ) -> Self {
        Self {
            id: id.into(),
            language,
            profile,
            prompt: prompt.into(),
            tags,
            baseline_quality: clamp_unit(baseline_quality),
            baseline_latency_ms: 40,
            baseline_token_proxy: baseline_token_proxy.max(1),
            validation: SelfEvolvingMemoryValidationEvidence::passed(),
        }
    }

    pub fn with_baseline_latency_ms(mut self, baseline_latency_ms: u128) -> Self {
        self.baseline_latency_ms = baseline_latency_ms.max(1);
        self
    }

    pub fn with_validation(mut self, validation: SelfEvolvingMemoryValidationEvidence) -> Self {
        self.validation = validation;
        self
    }

    fn prompt_digest(&self) -> String {
        stable_digest(&format!("{}:{}", self.id, self.prompt))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelfEvolvingMemoryAbHarness {
    pub modes: Vec<SelfEvolvingMemoryEvalMode>,
    pub record_limit: usize,
    pub token_budget: usize,
    pub improvement_epsilon: f32,
}

impl Default for SelfEvolvingMemoryAbHarness {
    fn default() -> Self {
        Self {
            modes: SelfEvolvingMemoryEvalMode::all(),
            record_limit: 6,
            token_budget: 192,
            improvement_epsilon: 0.005,
        }
    }
}

impl SelfEvolvingMemoryAbHarness {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run(
        &self,
        store: &SelfEvolvingMemoryStore,
        cases: &[SelfEvolvingMemoryAbCase],
    ) -> SelfEvolvingMemoryAbReport {
        let mut results = Vec::new();
        let mut failures = Vec::new();
        if cases.is_empty() {
            failures.push("self_evolving_memory_ab_cases_missing".to_owned());
        }
        if self.modes.is_empty() {
            failures.push("self_evolving_memory_ab_modes_missing".to_owned());
        }

        for case in cases {
            validate_case(case, &mut failures);
            let retrieval = store.retrieve_context(&SelfEvolvingMemoryQuery {
                prompt: case.prompt.clone(),
                profile: case.profile,
                tags: case.tags.clone(),
                record_limit: self.record_limit,
                token_budget: self.token_budget,
            });

            for mode in &self.modes {
                results.push(evaluate_case_mode(
                    case,
                    *mode,
                    &retrieval,
                    self.improvement_epsilon,
                ));
            }
        }

        let report = SelfEvolvingMemoryAbReport { results, failures };
        report.validate()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelfEvolvingMemoryAbResult {
    pub case_digest: String,
    pub mode: SelfEvolvingMemoryEvalMode,
    pub language: SelfEvolvingMemoryEvalLanguage,
    pub profile: TaskProfile,
    pub baseline_quality: f32,
    pub quality: f32,
    pub quality_delta: f32,
    pub baseline_latency_ms: u128,
    pub latency_ms: u128,
    pub baseline_token_proxy: usize,
    pub token_proxy: usize,
    pub retrieved_episodes: usize,
    pub retrieved_heuristics: usize,
    pub retrieved_tools: usize,
    pub retrieved_records: usize,
    pub retrieved_token_proxy: usize,
    pub candidate_previews: usize,
    pub admitted_candidates: usize,
    pub unsafe_write_rejections: usize,
    pub compiler_passed: bool,
    pub tests_passed: bool,
    pub benchmark_passed: bool,
    pub preview_only: bool,
    pub recommendation: SelfEvolvingMemoryAbRecommendation,
    pub ledger_digest: String,
}

impl SelfEvolvingMemoryAbResult {
    pub fn token_savings(&self) -> usize {
        self.baseline_token_proxy.saturating_sub(self.token_proxy)
    }

    pub fn is_win(&self) -> bool {
        self.quality_delta > 0.0
    }

    pub fn is_regression(&self) -> bool {
        self.quality_delta < 0.0
    }

    pub fn validation_passed(&self) -> bool {
        self.compiler_passed && self.tests_passed && self.benchmark_passed
    }

    pub fn ledger_line(&self) -> String {
        format!(
            "self_evolving_memory_ab_v1 case={} mode={} language={} profile={:?} quality_delta={:.3} baseline_tokens={} tokens={} saved_tokens={} retrieved={} candidate_previews={} admitted={} unsafe_write_rejections={} compiler_passed={} tests_passed={} benchmark_passed={} preview_only={} recommendation={} ledger_digest={}",
            self.case_digest,
            self.mode.as_str(),
            self.language.as_str(),
            self.profile,
            self.quality_delta,
            self.baseline_token_proxy,
            self.token_proxy,
            self.token_savings(),
            self.retrieved_records,
            self.candidate_previews,
            self.admitted_candidates,
            self.unsafe_write_rejections,
            self.compiler_passed,
            self.tests_passed,
            self.benchmark_passed,
            self.preview_only,
            self.recommendation.as_str(),
            self.ledger_digest
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SelfEvolvingMemoryAbReport {
    pub results: Vec<SelfEvolvingMemoryAbResult>,
    pub failures: Vec<String>,
}

impl SelfEvolvingMemoryAbReport {
    pub fn result_count(&self) -> usize {
        self.results.len()
    }

    pub fn case_count(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.case_digest.as_str())
            .collect::<BTreeSet<_>>()
            .len()
    }

    pub fn mode_count(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.mode)
            .collect::<BTreeSet<_>>()
            .len()
    }

    pub fn language_count(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.language)
            .collect::<BTreeSet<_>>()
            .len()
    }

    pub fn baseline_results(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.mode == SelfEvolvingMemoryEvalMode::Baseline)
            .count()
    }

    pub fn memory_results(&self) -> usize {
        self.result_count().saturating_sub(self.baseline_results())
    }

    pub fn win_count(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.mode != SelfEvolvingMemoryEvalMode::Baseline)
            .filter(|result| result.is_win())
            .count()
    }

    pub fn regression_count(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.mode != SelfEvolvingMemoryEvalMode::Baseline)
            .filter(|result| result.is_regression())
            .count()
    }

    pub fn candidate_previews(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.candidate_previews)
            .sum()
    }

    pub fn admitted_candidates(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.admitted_candidates)
            .sum()
    }

    pub fn unsafe_write_rejections(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.unsafe_write_rejections)
            .sum()
    }

    pub fn rollback_recommendations(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.recommendation == SelfEvolvingMemoryAbRecommendation::Rollback)
            .count()
    }

    pub fn quarantine_recommendations(&self) -> usize {
        self.results
            .iter()
            .filter(|result| {
                result.recommendation == SelfEvolvingMemoryAbRecommendation::Quarantine
            })
            .count()
    }

    pub fn compiler_passed(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.compiler_passed)
            .count()
    }

    pub fn tests_passed(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.tests_passed)
            .count()
    }

    pub fn benchmark_passed(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.benchmark_passed)
            .count()
    }

    pub fn total_token_savings(&self) -> usize {
        self.results
            .iter()
            .map(SelfEvolvingMemoryAbResult::token_savings)
            .sum()
    }

    pub fn retrieved_records(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.retrieved_records)
            .sum()
    }

    pub fn average_quality_delta(&self) -> f32 {
        average(
            self.results
                .iter()
                .filter(|result| result.mode != SelfEvolvingMemoryEvalMode::Baseline)
                .map(|result| result.quality_delta),
        )
    }

    pub fn ledger_lines(&self) -> Vec<String> {
        self.results
            .iter()
            .map(SelfEvolvingMemoryAbResult::ledger_line)
            .collect()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "self_evolving_memory_ab results={} cases={} modes={} languages={} wins={} regressions={} retrieved={} token_savings={} candidate_previews={} admitted={} unsafe_write_rejections={} rollback_recommendations={} quarantine_recommendations={} compiler_passed={} tests_passed={} benchmark_passed={} failures={}",
            self.result_count(),
            self.case_count(),
            self.mode_count(),
            self.language_count(),
            self.win_count(),
            self.regression_count(),
            self.retrieved_records(),
            self.total_token_savings(),
            self.candidate_previews(),
            self.admitted_candidates(),
            self.unsafe_write_rejections(),
            self.rollback_recommendations(),
            self.quarantine_recommendations(),
            self.compiler_passed(),
            self.tests_passed(),
            self.benchmark_passed(),
            self.failures.len()
        )
    }

    pub fn json_line(&self) -> String {
        let evidence_digest = stable_digest(&self.ledger_lines().join("\n"));
        format!(
            "{{\"schema\":\"rust-norion-self-evolving-memory-store-v1\",\"operation\":\"ab_evaluation\",\"results\":{},\"cases\":{},\"modes\":{},\"languages\":{},\"wins\":{},\"regressions\":{},\"retrieved\":{},\"token_savings\":{},\"candidate_previews\":{},\"admitted_candidates\":{},\"unsafe_write_rejections\":{},\"rollback_recommendations\":{},\"quarantine_recommendations\":{},\"compiler_passed\":{},\"tests_passed\":{},\"benchmark_passed\":{},\"failures\":{},\"redacted\":true,\"report_only\":true,\"read_only\":true,\"write_allowed\":false,\"durable_write_allowed\":false,\"applied\":false,\"applied_to_disk\":false,\"evidence_digest\":\"{}\"}}",
            self.result_count(),
            self.case_count(),
            self.mode_count(),
            self.language_count(),
            self.win_count(),
            self.regression_count(),
            self.retrieved_records(),
            self.total_token_savings(),
            self.candidate_previews(),
            self.admitted_candidates(),
            self.unsafe_write_rejections(),
            self.rollback_recommendations(),
            self.quarantine_recommendations(),
            self.compiler_passed(),
            self.tests_passed(),
            self.benchmark_passed(),
            self.failures.len(),
            evidence_digest
        )
    }

    pub fn evaluate(&self, gate: &SelfEvolvingMemoryAbGate) -> SelfEvolvingMemoryAbGateReport {
        let mut failures = self.failures.clone();
        require_at_least(&mut failures, "cases", self.case_count(), gate.min_cases);
        require_at_least(
            &mut failures,
            "languages",
            self.language_count(),
            gate.min_languages,
        );
        require_at_least(&mut failures, "wins", self.win_count(), gate.min_wins);
        require_at_least(
            &mut failures,
            "regressions",
            self.regression_count(),
            gate.min_regressions,
        );
        require_at_least(
            &mut failures,
            "token_savings",
            self.total_token_savings(),
            gate.min_token_savings,
        );
        require_at_least(
            &mut failures,
            "candidate_previews",
            self.candidate_previews(),
            gate.min_candidate_previews,
        );
        require_at_least(
            &mut failures,
            "unsafe_write_rejections",
            self.unsafe_write_rejections(),
            gate.min_unsafe_write_rejections,
        );
        require_at_least(
            &mut failures,
            "compiler_passed",
            self.compiler_passed(),
            gate.min_compiler_passed,
        );
        require_at_least(
            &mut failures,
            "tests_passed",
            self.tests_passed(),
            gate.min_tests_passed,
        );
        require_at_least(
            &mut failures,
            "benchmark_passed",
            self.benchmark_passed(),
            gate.min_benchmark_passed,
        );

        if gate.require_baseline && self.baseline_results() < self.case_count() {
            failures.push(format!(
                "baseline coverage {} below cases {}",
                self.baseline_results(),
                self.case_count()
            ));
        }
        if gate.require_all_memory_modes {
            for mode in [
                SelfEvolvingMemoryEvalMode::Episodic,
                SelfEvolvingMemoryEvalMode::Heuristic,
                SelfEvolvingMemoryEvalMode::ToolReliability,
                SelfEvolvingMemoryEvalMode::Combined,
            ] {
                if !self.results.iter().any(|result| result.mode == mode) {
                    failures.push(format!("missing memory mode {}", mode.as_str()));
                }
            }
        }
        if self.admitted_candidates() > gate.max_admitted_candidates {
            failures.push(format!(
                "admitted_candidates {} exceeds max {}",
                self.admitted_candidates(),
                gate.max_admitted_candidates
            ));
        }
        if self.failures.len() > gate.max_failures {
            failures.push(format!(
                "failures {} exceeds max {}",
                self.failures.len(),
                gate.max_failures
            ));
        }
        if gate.require_digest_only_ledger && !self.ledger_is_digest_only() {
            failures.push("ledger contains non-digest payload markers".to_owned());
        }

        SelfEvolvingMemoryAbGateReport {
            passed: failures.is_empty(),
            failures,
        }
    }

    pub fn ledger_is_digest_only(&self) -> bool {
        self.ledger_lines().iter().all(|line| {
            !contains_private_or_executable_marker(line)
                && !line.contains("solution:")
                && !line.contains("cargo test fails")
                && !line.contains("请")
                && !line.contains("Explain")
        })
    }

    fn validate(mut self) -> Self {
        for result in &self.results {
            if result.mode == SelfEvolvingMemoryEvalMode::Baseline {
                if result.candidate_previews > 0 || result.retrieved_records > 0 {
                    self.failures.push(format!(
                        "{} baseline result must not retrieve memory or create candidates",
                        result.case_digest
                    ));
                }
            } else {
                if !result.preview_only || result.admitted_candidates > 0 {
                    self.failures.push(format!(
                        "{}:{} memory candidate must remain preview-only and unadmitted",
                        result.case_digest,
                        result.mode.as_str()
                    ));
                }
                if result.is_regression()
                    && !matches!(
                        result.recommendation,
                        SelfEvolvingMemoryAbRecommendation::Quarantine
                            | SelfEvolvingMemoryAbRecommendation::Rollback
                    )
                {
                    self.failures.push(format!(
                        "{}:{} regression requires rollback or quarantine recommendation",
                        result.case_digest,
                        result.mode.as_str()
                    ));
                }
            }
        }
        if !self.ledger_is_digest_only() {
            self.failures
                .push("self_evolving_memory_ab_ledger_not_digest_only".to_owned());
        }
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelfEvolvingMemoryAbGate {
    pub min_cases: usize,
    pub require_baseline: bool,
    pub require_all_memory_modes: bool,
    pub min_languages: usize,
    pub min_wins: usize,
    pub min_regressions: usize,
    pub min_token_savings: usize,
    pub min_candidate_previews: usize,
    pub max_admitted_candidates: usize,
    pub min_unsafe_write_rejections: usize,
    pub min_compiler_passed: usize,
    pub min_tests_passed: usize,
    pub min_benchmark_passed: usize,
    pub max_failures: usize,
    pub require_digest_only_ledger: bool,
}

impl Default for SelfEvolvingMemoryAbGate {
    fn default() -> Self {
        Self {
            min_cases: 3,
            require_baseline: true,
            require_all_memory_modes: true,
            min_languages: 3,
            min_wins: 3,
            min_regressions: 1,
            min_token_savings: 1,
            min_candidate_previews: 1,
            max_admitted_candidates: 0,
            min_unsafe_write_rejections: 1,
            min_compiler_passed: 1,
            min_tests_passed: 1,
            min_benchmark_passed: 1,
            max_failures: 0,
            require_digest_only_ledger: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfEvolvingMemoryAbGateReport {
    pub passed: bool,
    pub failures: Vec<String>,
}

impl SelfEvolvingMemoryAbGateReport {
    pub fn summary_line(&self) -> String {
        format!(
            "self_evolving_memory_ab_gate passed={} failures={}",
            self.passed,
            self.failures.len()
        )
    }
}

pub fn run_default_self_evolving_memory_ab_suite() -> SelfEvolvingMemoryAbReport {
    let store = seeded_self_evolving_memory_ab_store();
    SelfEvolvingMemoryAbHarness::new().run(&store, &default_self_evolving_memory_ab_cases())
}

pub fn seeded_self_evolving_memory_ab_store() -> SelfEvolvingMemoryStore {
    let mut store = SelfEvolvingMemoryStore::new();
    let approval = SelfEvolvingMemoryApproval::approved(
        "rollback:benchmark:self-evolving-memory",
        vec![
            "compiler:passed".to_owned(),
            "tests:passed".to_owned(),
            "benchmark:passed".to_owned(),
        ],
    );

    store.append_episode(
        SelfEvolvingEpisodeInput {
            problem: "Explain a local memory architecture without leaking private state."
                .to_owned(),
            solution_path: "Summarize stable components, gates, and rollback evidence.".to_owned(),
            outcome: "Architecture explanation stays concise and evidence-backed.".to_owned(),
            key_insights: vec!["Prefer digest-only evidence for public summaries.".to_owned()],
            tags: vec![
                "english".to_owned(),
                "architecture".to_owned(),
                "memory".to_owned(),
            ],
            profile: TaskProfile::Coding,
            quality: 0.90,
            token_estimate: 36,
            source_case_id: "benchmark:english-memory-architecture".to_owned(),
        },
        &approval,
    );
    store.append_episode(
        SelfEvolvingEpisodeInput {
            problem: "Fix a Rust test failure while preserving rollback evidence.".to_owned(),
            solution_path: "Run cargo test, isolate the failing assertion, and patch narrowly."
                .to_owned(),
            outcome: "Focused Rust fix passed compiler and regression checks.".to_owned(),
            key_insights: vec![
                "Use the smallest passing Rust regression before refactor.".to_owned(),
            ],
            tags: vec![
                "rust".to_owned(),
                "test".to_owned(),
                "cargo-test".to_owned(),
            ],
            profile: TaskProfile::Coding,
            quality: 0.94,
            token_estimate: 48,
            source_case_id: "benchmark:rust-cargo-test".to_owned(),
        },
        &approval,
    );
    store.append_heuristic(
        SelfEvolvingHeuristicInput {
            rule: "For Chinese Rust explanations, keep terms bilingual and validate code paths."
                .to_owned(),
            tags: vec![
                "chinese".to_owned(),
                "rust".to_owned(),
                "explain".to_owned(),
            ],
            profile: TaskProfile::Coding,
            priority: 0.82,
            confidence: 0.88,
            source_case_id: "benchmark:chinese-rust-explain".to_owned(),
            updated_step: 4,
        },
        &approval,
    );
    store.append_heuristic(
        SelfEvolvingHeuristicInput {
            rule: "When cargo test is the validator, cite compiler/test/benchmark evidence."
                .to_owned(),
            tags: vec![
                "rust".to_owned(),
                "test".to_owned(),
                "cargo-test".to_owned(),
            ],
            profile: TaskProfile::Coding,
            priority: 0.86,
            confidence: 0.90,
            source_case_id: "benchmark:rust-validation-heuristic".to_owned(),
            updated_step: 5,
        },
        &approval,
    );
    store.observe_tool(
        ToolReliabilityObservationInput {
            tool_name: "cargo-test".to_owned(),
            profile: TaskProfile::Coding,
            success: true,
            quality: 0.92,
            source_case_id: "benchmark:cargo-test-green".to_owned(),
            observed_step: 6,
        },
        &approval,
    );
    store.observe_tool(
        ToolReliabilityObservationInput {
            tool_name: "cargo-test".to_owned(),
            profile: TaskProfile::Coding,
            success: true,
            quality: 0.88,
            source_case_id: "benchmark:cargo-test-green-2".to_owned(),
            observed_step: 7,
        },
        &approval,
    );
    store.observe_tool(
        ToolReliabilityObservationInput {
            tool_name: "speculative-patcher".to_owned(),
            profile: TaskProfile::Coding,
            success: false,
            quality: 0.10,
            source_case_id: "benchmark:speculative-patcher-red".to_owned(),
            observed_step: 8,
        },
        &approval,
    );

    store
}

pub fn default_self_evolving_memory_ab_cases() -> Vec<SelfEvolvingMemoryAbCase> {
    vec![
        SelfEvolvingMemoryAbCase::new(
            "english-memory-architecture",
            SelfEvolvingMemoryEvalLanguage::English,
            TaskProfile::Coding,
            "Explain how the local Rust memory engine should reuse approved experience.",
            vec![
                "english".to_owned(),
                "architecture".to_owned(),
                "memory".to_owned(),
            ],
            0.70,
            240,
        )
        .with_baseline_latency_ms(42),
        SelfEvolvingMemoryAbCase::new(
            "chinese-rust-explain",
            SelfEvolvingMemoryEvalLanguage::Chinese,
            TaskProfile::Coding,
            "请用中文解释 Rust 测试失败时如何复用经过验证的记忆。",
            vec![
                "chinese".to_owned(),
                "rust".to_owned(),
                "explain".to_owned(),
            ],
            0.68,
            260,
        )
        .with_baseline_latency_ms(45),
        SelfEvolvingMemoryAbCase::new(
            "rust-cargo-test-fix",
            SelfEvolvingMemoryEvalLanguage::RustCoding,
            TaskProfile::Coding,
            "cargo test fails in a Rust module; choose a focused compiler-backed repair.",
            vec![
                "rust".to_owned(),
                "test".to_owned(),
                "cargo-test".to_owned(),
            ],
            0.72,
            280,
        )
        .with_baseline_latency_ms(48),
        SelfEvolvingMemoryAbCase::new(
            "rust-unsafe-tool-regression",
            SelfEvolvingMemoryEvalLanguage::RustCoding,
            TaskProfile::Coding,
            "A speculative patcher suggests an unsafe Rust edit without passing tests.",
            vec![
                "rust".to_owned(),
                "unsafe".to_owned(),
                "speculative-patcher".to_owned(),
            ],
            0.74,
            220,
        )
        .with_baseline_latency_ms(44)
        .with_validation(SelfEvolvingMemoryValidationEvidence::failed()),
    ]
}

fn evaluate_case_mode(
    case: &SelfEvolvingMemoryAbCase,
    mode: SelfEvolvingMemoryEvalMode,
    retrieval: &SelfEvolvingMemoryRetrievalReport,
    improvement_epsilon: f32,
) -> SelfEvolvingMemoryAbResult {
    if mode == SelfEvolvingMemoryEvalMode::Baseline {
        return result_for_baseline(case);
    }

    let retrieved_episodes = usize::from(mode.includes_episode()) * retrieval.episodes.len();
    let retrieved_heuristics = usize::from(mode.includes_heuristic()) * retrieval.heuristics.len();
    let retrieved_tools =
        usize::from(mode.includes_tool_reliability()) * retrieval.tool_reliability.len();
    let retrieved_records = retrieved_episodes
        .saturating_add(retrieved_heuristics)
        .saturating_add(retrieved_tools);
    let retrieved_token_proxy = retained_tokens_for_mode(mode, retrieval);
    let episode_delta = if mode.includes_episode() {
        retrieval
            .episodes
            .iter()
            .map(|context| context.score * 0.035)
            .sum::<f32>()
    } else {
        0.0
    };
    let heuristic_delta = if mode.includes_heuristic() {
        retrieval
            .heuristics
            .iter()
            .map(|context| context.score * 0.030)
            .sum::<f32>()
    } else {
        0.0
    };
    let tool_delta = if mode.includes_tool_reliability() {
        retrieval
            .tool_reliability
            .iter()
            .filter(|tool| tool_matches_case(tool.tool_id.as_str(), &case.tags))
            .map(|tool| (tool.trust_score - 0.50) * 0.12)
            .sum::<f32>()
    } else {
        0.0
    };
    let synergy_delta = if mode == SelfEvolvingMemoryEvalMode::Combined
        && retrieved_episodes > 0
        && retrieved_heuristics > 0
    {
        0.015
    } else {
        0.0
    };
    let validation_penalty = if case.validation.all_passed() {
        0.0
    } else {
        0.045
    };
    let quality_delta = (episode_delta + heuristic_delta + tool_delta + synergy_delta
        - validation_penalty)
        .clamp(-0.25, 0.25);
    let quality = clamp_unit(case.baseline_quality + quality_delta);
    let token_proxy = memory_token_proxy(
        case.baseline_token_proxy,
        retrieved_token_proxy,
        quality_delta,
    );
    let latency_ms = memory_latency_ms(
        case.baseline_latency_ms,
        retrieved_records,
        quality_delta,
        mode == SelfEvolvingMemoryEvalMode::Combined,
    );
    let candidate_previews = usize::from(retrieved_records > 0);
    let validation = if quality_delta > improvement_epsilon && case.validation.all_passed() {
        case.validation
    } else {
        SelfEvolvingMemoryValidationEvidence::failed()
    };
    let recommendation = recommendation_for(
        quality_delta,
        tool_delta,
        retrieved_records,
        improvement_epsilon,
        validation,
    );
    let unsafe_write_rejections = usize::from(matches!(
        recommendation,
        SelfEvolvingMemoryAbRecommendation::Quarantine
            | SelfEvolvingMemoryAbRecommendation::Rollback
    ));
    let case_digest = case.prompt_digest();
    let ledger_digest = stable_digest(&format!(
        "{}:{}:{:.3}:{}:{}:{}",
        case_digest,
        mode.as_str(),
        quality_delta,
        retrieved_records,
        token_proxy,
        recommendation.as_str()
    ));

    SelfEvolvingMemoryAbResult {
        case_digest,
        mode,
        language: case.language,
        profile: case.profile,
        baseline_quality: case.baseline_quality,
        quality,
        quality_delta,
        baseline_latency_ms: case.baseline_latency_ms,
        latency_ms,
        baseline_token_proxy: case.baseline_token_proxy,
        token_proxy,
        retrieved_episodes,
        retrieved_heuristics,
        retrieved_tools,
        retrieved_records,
        retrieved_token_proxy,
        candidate_previews,
        admitted_candidates: 0,
        unsafe_write_rejections,
        compiler_passed: validation.compiler_passed,
        tests_passed: validation.tests_passed,
        benchmark_passed: validation.benchmark_passed,
        preview_only: true,
        recommendation,
        ledger_digest,
    }
}

fn result_for_baseline(case: &SelfEvolvingMemoryAbCase) -> SelfEvolvingMemoryAbResult {
    let case_digest = case.prompt_digest();
    let ledger_digest = stable_digest(&format!("{}:baseline", case_digest));
    SelfEvolvingMemoryAbResult {
        case_digest,
        mode: SelfEvolvingMemoryEvalMode::Baseline,
        language: case.language,
        profile: case.profile,
        baseline_quality: case.baseline_quality,
        quality: case.baseline_quality,
        quality_delta: 0.0,
        baseline_latency_ms: case.baseline_latency_ms,
        latency_ms: case.baseline_latency_ms,
        baseline_token_proxy: case.baseline_token_proxy,
        token_proxy: case.baseline_token_proxy,
        retrieved_episodes: 0,
        retrieved_heuristics: 0,
        retrieved_tools: 0,
        retrieved_records: 0,
        retrieved_token_proxy: 0,
        candidate_previews: 0,
        admitted_candidates: 0,
        unsafe_write_rejections: 0,
        compiler_passed: false,
        tests_passed: false,
        benchmark_passed: false,
        preview_only: true,
        recommendation: SelfEvolvingMemoryAbRecommendation::Noop,
        ledger_digest,
    }
}

fn retained_tokens_for_mode(
    mode: SelfEvolvingMemoryEvalMode,
    retrieval: &SelfEvolvingMemoryRetrievalReport,
) -> usize {
    let episodes = if mode.includes_episode() {
        retrieval
            .episodes
            .iter()
            .map(|context| context.token_estimate)
            .sum()
    } else {
        0
    };
    let heuristics = if mode.includes_heuristic() {
        retrieval
            .heuristics
            .iter()
            .map(|context| context.token_estimate)
            .sum()
    } else {
        0
    };
    let tools = if mode.includes_tool_reliability() {
        retrieval
            .tool_reliability
            .iter()
            .map(|context| context.token_estimate)
            .sum()
    } else {
        0
    };
    episodes + heuristics + tools
}

fn memory_token_proxy(
    baseline_token_proxy: usize,
    retrieved_token_proxy: usize,
    quality_delta: f32,
) -> usize {
    let gross_savings = if quality_delta > 0.0 {
        ((quality_delta * 900.0) as usize).saturating_add(retrieved_token_proxy / 2)
    } else {
        0
    };
    baseline_token_proxy
        .saturating_add(retrieved_token_proxy)
        .saturating_sub(gross_savings)
        .max(1)
}

fn memory_latency_ms(
    baseline_latency_ms: u128,
    retrieved_records: usize,
    quality_delta: f32,
    combined: bool,
) -> u128 {
    let retrieval_overhead = retrieved_records as u128;
    let savings = if quality_delta > 0.0 {
        ((quality_delta * 60.0) as u128).saturating_add(u128::from(combined))
    } else {
        0
    };
    baseline_latency_ms
        .saturating_add(retrieval_overhead)
        .saturating_sub(savings)
        .max(1)
}

fn recommendation_for(
    quality_delta: f32,
    tool_delta: f32,
    retrieved_records: usize,
    improvement_epsilon: f32,
    validation: SelfEvolvingMemoryValidationEvidence,
) -> SelfEvolvingMemoryAbRecommendation {
    if retrieved_records == 0 {
        return SelfEvolvingMemoryAbRecommendation::HoldForEvidence;
    }
    if quality_delta < -improvement_epsilon || tool_delta < -improvement_epsilon {
        return SelfEvolvingMemoryAbRecommendation::Quarantine;
    }
    if !validation.all_passed() {
        return SelfEvolvingMemoryAbRecommendation::Rollback;
    }
    if quality_delta > improvement_epsilon {
        SelfEvolvingMemoryAbRecommendation::HoldForApproval
    } else {
        SelfEvolvingMemoryAbRecommendation::HoldForEvidence
    }
}

fn tool_matches_case(tool_id: &str, tags: &[String]) -> bool {
    let tool_id = tool_id.to_ascii_lowercase();
    tags.iter()
        .map(|tag| tag.to_ascii_lowercase())
        .any(|tag| tag == tool_id || tag.replace('_', "-") == tool_id)
}

fn validate_case(case: &SelfEvolvingMemoryAbCase, failures: &mut Vec<String>) {
    if case.id.trim().is_empty() {
        failures.push("self_evolving_memory_ab_case_id_missing".to_owned());
    }
    if case.prompt.trim().is_empty() {
        failures.push(format!("{} prompt missing", case.id));
    }
    if case.tags.is_empty() {
        failures.push(format!("{} tags missing", case.id));
    }
}

fn require_at_least(failures: &mut Vec<String>, metric: &str, actual: usize, required: usize) {
    if actual < required {
        failures.push(format!("{metric} {actual} below required {required}"));
    }
}

fn average(values: impl Iterator<Item = f32>) -> f32 {
    let mut total = 0.0;
    let mut count = 0usize;
    for value in values {
        total += value;
        count += 1;
    }
    if count == 0 {
        0.0
    } else {
        total / count as f32
    }
}

fn clamp_unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn stable_digest(value: &str) -> String {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash = FNV_OFFSET;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("fnv64:{hash:016x}")
}
