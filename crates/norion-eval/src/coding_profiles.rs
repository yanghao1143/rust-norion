use std::collections::BTreeMap;

use norion_test::{VerificationPhase, VerificationPlan};

pub const CODING_EVAL_SCHEMA_VERSION: &str = "coding_eval_v1";
pub const CODING_EVAL_PROVENANCE: &str = "synthetic-local-noncommercial-fixtures-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CodingEvalProfileKind {
    EnglishInstruction,
    ChineseInstruction,
    RustCodeGeneration,
    RustRepair,
    MultilingualCodingExplanation,
}

impl CodingEvalProfileKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EnglishInstruction => "english_instruction",
            Self::ChineseInstruction => "chinese_instruction",
            Self::RustCodeGeneration => "rust_code_generation",
            Self::RustRepair => "rust_repair",
            Self::MultilingualCodingExplanation => "multilingual_coding_explanation",
        }
    }

    pub fn expected_profiles() -> [Self; 5] {
        [
            Self::EnglishInstruction,
            Self::ChineseInstruction,
            Self::RustCodeGeneration,
            Self::RustRepair,
            Self::MultilingualCodingExplanation,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CodingEvalFailureCategory {
    MissingExpectedMarker,
    ForbiddenMarker,
    CompileCheckMissing,
    CompileCheckFailed,
    UnitTestMissing,
    UnitTestFailed,
    BenchmarkRegression,
    MemoryMiss,
    TokenBudgetExceeded,
    LatencyBudgetExceeded,
    RedactionViolation,
    OverfitRisk,
    UnsafeFixtureProvenance,
}

impl CodingEvalFailureCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MissingExpectedMarker => "missing_expected_marker",
            Self::ForbiddenMarker => "forbidden_marker",
            Self::CompileCheckMissing => "compile_check_missing",
            Self::CompileCheckFailed => "compile_check_failed",
            Self::UnitTestMissing => "unit_test_missing",
            Self::UnitTestFailed => "unit_test_failed",
            Self::BenchmarkRegression => "benchmark_regression",
            Self::MemoryMiss => "memory_miss",
            Self::TokenBudgetExceeded => "token_budget_exceeded",
            Self::LatencyBudgetExceeded => "latency_budget_exceeded",
            Self::RedactionViolation => "redaction_violation",
            Self::OverfitRisk => "overfit_risk",
            Self::UnsafeFixtureProvenance => "unsafe_fixture_provenance",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CodingEvalThresholds {
    pub min_marker_coverage: f32,
    pub require_compile_check: bool,
    pub require_unit_tests: bool,
    pub max_benchmark_regression_pct: f32,
    pub min_memory_hit_rate: f32,
    pub max_tokens: u64,
    pub max_latency_ms: u64,
    pub require_redaction: bool,
}

impl CodingEvalThresholds {
    pub fn for_profile(profile: CodingEvalProfileKind) -> Self {
        match profile {
            CodingEvalProfileKind::EnglishInstruction => Self {
                min_marker_coverage: 0.75,
                require_compile_check: false,
                require_unit_tests: false,
                max_benchmark_regression_pct: 5.0,
                min_memory_hit_rate: 0.20,
                max_tokens: 900,
                max_latency_ms: 4_000,
                require_redaction: true,
            },
            CodingEvalProfileKind::ChineseInstruction => Self {
                min_marker_coverage: 0.75,
                require_compile_check: false,
                require_unit_tests: false,
                max_benchmark_regression_pct: 5.0,
                min_memory_hit_rate: 0.20,
                max_tokens: 900,
                max_latency_ms: 4_000,
                require_redaction: true,
            },
            CodingEvalProfileKind::RustCodeGeneration => Self {
                min_marker_coverage: 0.80,
                require_compile_check: true,
                require_unit_tests: true,
                max_benchmark_regression_pct: 2.0,
                min_memory_hit_rate: 0.35,
                max_tokens: 1_600,
                max_latency_ms: 6_000,
                require_redaction: true,
            },
            CodingEvalProfileKind::RustRepair => Self {
                min_marker_coverage: 0.80,
                require_compile_check: true,
                require_unit_tests: true,
                max_benchmark_regression_pct: 2.0,
                min_memory_hit_rate: 0.40,
                max_tokens: 1_800,
                max_latency_ms: 6_500,
                require_redaction: true,
            },
            CodingEvalProfileKind::MultilingualCodingExplanation => Self {
                min_marker_coverage: 0.80,
                require_compile_check: false,
                require_unit_tests: false,
                max_benchmark_regression_pct: 3.0,
                min_memory_hit_rate: 0.30,
                max_tokens: 1_200,
                max_latency_ms: 5_000,
                require_redaction: true,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CodingEvalScoringProfile {
    pub kind: CodingEvalProfileKind,
    pub thresholds: CodingEvalThresholds,
    pub failure_categories: Vec<CodingEvalFailureCategory>,
}

impl CodingEvalScoringProfile {
    pub fn for_kind(kind: CodingEvalProfileKind) -> Self {
        Self {
            kind,
            thresholds: CodingEvalThresholds::for_profile(kind),
            failure_categories: vec![
                CodingEvalFailureCategory::MissingExpectedMarker,
                CodingEvalFailureCategory::ForbiddenMarker,
                CodingEvalFailureCategory::CompileCheckMissing,
                CodingEvalFailureCategory::CompileCheckFailed,
                CodingEvalFailureCategory::UnitTestMissing,
                CodingEvalFailureCategory::UnitTestFailed,
                CodingEvalFailureCategory::BenchmarkRegression,
                CodingEvalFailureCategory::MemoryMiss,
                CodingEvalFailureCategory::TokenBudgetExceeded,
                CodingEvalFailureCategory::LatencyBudgetExceeded,
                CodingEvalFailureCategory::RedactionViolation,
                CodingEvalFailureCategory::OverfitRisk,
                CodingEvalFailureCategory::UnsafeFixtureProvenance,
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodingEvalFixture {
    pub id: String,
    pub profile: CodingEvalProfileKind,
    pub prompt: String,
    pub expected_markers: Vec<String>,
    pub forbidden_markers: Vec<String>,
    pub validation_plans: Vec<VerificationPlan>,
    pub provenance: String,
    pub license_safe: bool,
    pub private_source: bool,
    pub overfit_guard_markers: Vec<String>,
}

impl CodingEvalFixture {
    pub fn new(
        id: impl Into<String>,
        profile: CodingEvalProfileKind,
        prompt: impl Into<String>,
        expected_markers: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            id: id.into(),
            profile,
            prompt: prompt.into(),
            expected_markers: expected_markers.into_iter().map(Into::into).collect(),
            forbidden_markers: default_forbidden_markers(),
            validation_plans: Vec::new(),
            provenance: CODING_EVAL_PROVENANCE.to_owned(),
            license_safe: true,
            private_source: false,
            overfit_guard_markers: vec![
                "fixture-id".to_owned(),
                "memorized benchmark".to_owned(),
                "hard-coded answer".to_owned(),
            ],
        }
    }

    pub fn with_validation_plans(
        mut self,
        validation_plans: impl IntoIterator<Item = VerificationPlan>,
    ) -> Self {
        self.validation_plans = validation_plans.into_iter().collect();
        self
    }

    pub fn with_forbidden_markers(
        mut self,
        forbidden_markers: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.forbidden_markers = forbidden_markers.into_iter().map(Into::into).collect();
        self
    }

    pub fn prompt_digest(&self) -> String {
        stable_redaction_digest(["coding-eval-prompt", self.id.as_str(), self.prompt.as_str()])
    }

    pub fn requires_cargo_check(&self) -> bool {
        self.validation_plans.iter().any(plan_contains_cargo_check)
    }

    pub fn requires_cargo_test(&self) -> bool {
        self.validation_plans.iter().any(plan_contains_cargo_test)
    }

    pub fn provenance_safe(&self) -> bool {
        self.license_safe
            && !self.private_source
            && self.provenance == CODING_EVAL_PROVENANCE
            && !contains_private_or_unlicensed_marker(&self.prompt)
            && !self
                .expected_markers
                .iter()
                .any(|marker| contains_private_or_unlicensed_marker(marker))
    }

    pub fn validation_command_lines(&self) -> Vec<String> {
        self.validation_plans
            .iter()
            .flat_map(|plan| {
                plan.commands_for_phase(VerificationPhase::Both)
                    .into_iter()
                    .map(|command| command.display_line())
                    .collect::<Vec<_>>()
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CodingEvalObservation {
    pub fixture_id: String,
    pub output: String,
    pub compile_checked: bool,
    pub compile_passed: bool,
    pub unit_test_checked: bool,
    pub unit_test_passed: bool,
    pub benchmark_regression_pct: f32,
    pub memory_hit_rate: f32,
    pub tokens: u64,
    pub latency_ms: u64,
    pub redaction_passed: bool,
    pub overfit_signature_seen: bool,
}

impl CodingEvalObservation {
    pub fn new(fixture_id: impl Into<String>, output: impl Into<String>) -> Self {
        Self {
            fixture_id: fixture_id.into(),
            output: output.into(),
            compile_checked: false,
            compile_passed: false,
            unit_test_checked: false,
            unit_test_passed: false,
            benchmark_regression_pct: 0.0,
            memory_hit_rate: 0.0,
            tokens: 0,
            latency_ms: 0,
            redaction_passed: true,
            overfit_signature_seen: false,
        }
    }

    pub fn with_compile(mut self, checked: bool, passed: bool) -> Self {
        self.compile_checked = checked;
        self.compile_passed = passed;
        self
    }

    pub fn with_unit_tests(mut self, checked: bool, passed: bool) -> Self {
        self.unit_test_checked = checked;
        self.unit_test_passed = passed;
        self
    }

    pub fn with_runtime_metrics(
        mut self,
        memory_hit_rate: f32,
        tokens: u64,
        latency_ms: u64,
    ) -> Self {
        self.memory_hit_rate = finite_unit(memory_hit_rate);
        self.tokens = tokens;
        self.latency_ms = latency_ms;
        self
    }

    pub fn with_benchmark_regression(mut self, benchmark_regression_pct: f32) -> Self {
        self.benchmark_regression_pct = finite_nonnegative(benchmark_regression_pct);
        self
    }

    pub fn with_redaction(mut self, redaction_passed: bool) -> Self {
        self.redaction_passed = redaction_passed;
        self
    }

    pub fn with_overfit_signature(mut self) -> Self {
        self.overfit_signature_seen = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CodingEvalResult {
    pub fixture_id: String,
    pub profile: CodingEvalProfileKind,
    pub marker_coverage: f32,
    pub score: f32,
    pub passed: bool,
    pub failure_categories: Vec<CodingEvalFailureCategory>,
    pub evidence_packet: CodingEvalEvidencePacket,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CodingEvalEvidencePacket {
    pub schema_version: &'static str,
    pub fixture_id: String,
    pub profile: CodingEvalProfileKind,
    pub prompt_digest: String,
    pub output_digest: String,
    pub validation_commands: Vec<String>,
    pub marker_coverage: f32,
    pub compile_checked: bool,
    pub compile_passed: bool,
    pub unit_test_checked: bool,
    pub unit_test_passed: bool,
    pub benchmark_regression_pct: f32,
    pub memory_hit_rate: f32,
    pub tokens: u64,
    pub latency_ms: u64,
    pub redaction_passed: bool,
    pub overfit_signature_seen: bool,
    pub failure_categories: Vec<CodingEvalFailureCategory>,
    pub provenance_digest: String,
}

impl CodingEvalEvidencePacket {
    pub fn record_line(&self) -> String {
        [
            self.schema_version.to_owned(),
            self.fixture_id.clone(),
            self.profile.as_str().to_owned(),
            self.prompt_digest.clone(),
            self.output_digest.clone(),
            self.validation_commands.join("|"),
            format!("{:.3}", self.marker_coverage),
            bool_field(self.compile_checked).to_owned(),
            bool_field(self.compile_passed).to_owned(),
            bool_field(self.unit_test_checked).to_owned(),
            bool_field(self.unit_test_passed).to_owned(),
            format!("{:.3}", self.benchmark_regression_pct),
            format!("{:.3}", self.memory_hit_rate),
            self.tokens.to_string(),
            self.latency_ms.to_string(),
            bool_field(self.redaction_passed).to_owned(),
            bool_field(self.overfit_signature_seen).to_owned(),
            self.failure_categories
                .iter()
                .map(|category| category.as_str())
                .collect::<Vec<_>>()
                .join("|"),
            self.provenance_digest.clone(),
        ]
        .iter()
        .map(|field| escape_field(field))
        .collect::<Vec<_>>()
        .join("\t")
    }

    pub fn summary_line(&self) -> String {
        format!(
            "coding_eval_packet schema={} fixture={} profile={} prompt={} output={} coverage={:.3} compile={}/{} test={}/{} bench_regression={:.3} memory_hit={:.3} tokens={} latency_ms={} redaction={} overfit={} failures={} provenance={}",
            self.schema_version,
            self.fixture_id,
            self.profile.as_str(),
            self.prompt_digest,
            self.output_digest,
            self.marker_coverage,
            self.compile_passed,
            self.compile_checked,
            self.unit_test_passed,
            self.unit_test_checked,
            self.benchmark_regression_pct,
            self.memory_hit_rate,
            self.tokens,
            self.latency_ms,
            self.redaction_passed,
            self.overfit_signature_seen,
            self.failure_categories
                .iter()
                .map(|category| category.as_str())
                .collect::<Vec<_>>()
                .join("|"),
            self.provenance_digest
        )
    }

    pub fn is_redacted(&self) -> bool {
        self.prompt_digest.starts_with("redaction-digest:")
            && self.output_digest.starts_with("redaction-digest:")
            && !contains_private_or_unlicensed_marker(&self.summary_line())
            && !contains_private_or_unlicensed_marker(&self.record_line())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CodingEvalCorpus {
    pub fixtures: Vec<CodingEvalFixture>,
    pub scoring_profiles: Vec<CodingEvalScoringProfile>,
}

impl Default for CodingEvalCorpus {
    fn default() -> Self {
        default_coding_eval_corpus()
    }
}

impl CodingEvalCorpus {
    pub fn profile(&self, profile: CodingEvalProfileKind) -> Option<&CodingEvalScoringProfile> {
        self.scoring_profiles
            .iter()
            .find(|scoring| scoring.kind == profile)
    }

    pub fn validate(&self) -> CodingEvalCorpusValidationReport {
        let mut failures = Vec::new();
        for profile in CodingEvalProfileKind::expected_profiles() {
            if !self
                .fixtures
                .iter()
                .any(|fixture| fixture.profile == profile)
            {
                failures.push(format!("missing_fixture_profile:{}", profile.as_str()));
            }
            if !self
                .scoring_profiles
                .iter()
                .any(|scoring| scoring.kind == profile)
            {
                failures.push(format!("missing_scoring_profile:{}", profile.as_str()));
            }
        }
        for fixture in &self.fixtures {
            if fixture.expected_markers.is_empty() {
                failures.push(format!("{}:missing_expected_markers", fixture.id));
            }
            if !fixture.provenance_safe() {
                failures.push(format!("{}:unsafe_provenance", fixture.id));
            }
            if matches!(
                fixture.profile,
                CodingEvalProfileKind::RustCodeGeneration | CodingEvalProfileKind::RustRepair
            ) && (!fixture.requires_cargo_check() || !fixture.requires_cargo_test())
            {
                failures.push(format!("{}:missing_cargo_check_or_test_plan", fixture.id));
            }
        }

        CodingEvalCorpusValidationReport {
            fixture_count: self.fixtures.len(),
            scoring_profile_count: self.scoring_profiles.len(),
            failures,
        }
    }

    pub fn score_observation(
        &self,
        observation: &CodingEvalObservation,
    ) -> Option<CodingEvalResult> {
        let fixture = self
            .fixtures
            .iter()
            .find(|fixture| fixture.id == observation.fixture_id)?;
        let profile = self.profile(fixture.profile)?;
        Some(score_fixture(fixture, profile, observation))
    }

    pub fn score_observations(
        &self,
        observations: &[CodingEvalObservation],
    ) -> CodingEvalSuiteReport {
        let results = observations
            .iter()
            .filter_map(|observation| self.score_observation(observation))
            .collect::<Vec<_>>();
        CodingEvalSuiteReport::from_results(results)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodingEvalCorpusValidationReport {
    pub fixture_count: usize,
    pub scoring_profile_count: usize,
    pub failures: Vec<String>,
}

impl CodingEvalCorpusValidationReport {
    pub fn passed(&self) -> bool {
        self.failures.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CodingEvalSuiteReport {
    pub schema_version: &'static str,
    pub result_count: usize,
    pub passed_count: usize,
    pub failed_count: usize,
    pub average_score: f32,
    pub profile_counts: BTreeMap<String, usize>,
    pub profile_pass_counts: BTreeMap<String, usize>,
    pub failure_category_counts: BTreeMap<String, usize>,
    pub overfit_suspect_count: usize,
    pub redaction_failure_count: usize,
    pub evidence_packets: Vec<CodingEvalEvidencePacket>,
}

impl CodingEvalSuiteReport {
    pub fn from_results(results: Vec<CodingEvalResult>) -> Self {
        let mut profile_counts = BTreeMap::new();
        let mut profile_pass_counts = BTreeMap::new();
        let mut failure_category_counts = BTreeMap::new();
        let mut score_sum = 0.0;
        let mut passed_count = 0usize;
        let mut overfit_suspect_count = 0usize;
        let mut redaction_failure_count = 0usize;

        for result in &results {
            score_sum += result.score;
            let profile = result.profile.as_str().to_owned();
            *profile_counts.entry(profile.clone()).or_insert(0) += 1;
            if result.passed {
                passed_count += 1;
                *profile_pass_counts.entry(profile).or_insert(0) += 1;
            }
            if result
                .failure_categories
                .contains(&CodingEvalFailureCategory::OverfitRisk)
            {
                overfit_suspect_count += 1;
            }
            if result
                .failure_categories
                .contains(&CodingEvalFailureCategory::RedactionViolation)
            {
                redaction_failure_count += 1;
            }
            for category in &result.failure_categories {
                *failure_category_counts
                    .entry(category.as_str().to_owned())
                    .or_insert(0) += 1;
            }
        }

        let result_count = results.len();
        Self {
            schema_version: CODING_EVAL_SCHEMA_VERSION,
            result_count,
            passed_count,
            failed_count: result_count.saturating_sub(passed_count),
            average_score: if result_count == 0 {
                0.0
            } else {
                score_sum / result_count as f32
            },
            profile_counts,
            profile_pass_counts,
            failure_category_counts,
            overfit_suspect_count,
            redaction_failure_count,
            evidence_packets: results
                .into_iter()
                .map(|result| result.evidence_packet)
                .collect(),
        }
    }

    pub fn profile_coverage(&self) -> usize {
        self.profile_counts.len()
    }

    pub fn pass_rate(&self) -> f32 {
        ratio(self.passed_count, self.result_count)
    }

    pub fn evidence_is_redacted(&self) -> bool {
        self.evidence_packets
            .iter()
            .all(CodingEvalEvidencePacket::is_redacted)
    }

    pub fn summary_line(&self) -> String {
        format!(
            "coding_eval_suite schema={} results={} passed={} failed={} pass_rate={:.3} avg_score={:.3} profiles={} overfit_suspect={} redaction_failures={} failures={} evidence_redacted={}",
            self.schema_version,
            self.result_count,
            self.passed_count,
            self.failed_count,
            self.pass_rate(),
            self.average_score,
            self.profile_coverage(),
            self.overfit_suspect_count,
            self.redaction_failure_count,
            map_summary(&self.failure_category_counts),
            self.evidence_is_redacted()
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CodingEvalImprovementReport {
    pub schema_version: &'static str,
    pub baseline_score: f32,
    pub candidate_score: f32,
    pub score_delta: f32,
    pub baseline_pass_rate: f32,
    pub candidate_pass_rate: f32,
    pub profile_coverage_delta: isize,
    pub overfit_suspected: bool,
    pub behavior_improved: bool,
    pub reasons: Vec<String>,
}

impl CodingEvalImprovementReport {
    pub fn compare(baseline: &CodingEvalSuiteReport, candidate: &CodingEvalSuiteReport) -> Self {
        let score_delta = candidate.average_score - baseline.average_score;
        let pass_delta = candidate.pass_rate() - baseline.pass_rate();
        let profile_coverage_delta =
            candidate.profile_coverage() as isize - baseline.profile_coverage() as isize;
        let overfit_suspected = candidate.overfit_suspect_count > baseline.overfit_suspect_count
            || candidate.profile_coverage() < baseline.profile_coverage()
            || !candidate.evidence_is_redacted();
        let behavior_improved = score_delta >= 0.05
            && pass_delta >= 0.0
            && !overfit_suspected
            && profile_coverage_delta >= 0;
        let mut reasons = Vec::new();
        if score_delta >= 0.05 {
            reasons.push("score_delta_met".to_owned());
        } else {
            reasons.push("score_delta_too_small".to_owned());
        }
        if pass_delta >= 0.0 {
            reasons.push("pass_rate_not_regressed".to_owned());
        } else {
            reasons.push("pass_rate_regressed".to_owned());
        }
        if overfit_suspected {
            reasons.push("overfit_or_redaction_risk".to_owned());
        }
        if profile_coverage_delta < 0 {
            reasons.push("profile_coverage_regressed".to_owned());
        }

        Self {
            schema_version: CODING_EVAL_SCHEMA_VERSION,
            baseline_score: baseline.average_score,
            candidate_score: candidate.average_score,
            score_delta,
            baseline_pass_rate: baseline.pass_rate(),
            candidate_pass_rate: candidate.pass_rate(),
            profile_coverage_delta,
            overfit_suspected,
            behavior_improved,
            reasons,
        }
    }

    pub fn summary_line(&self) -> String {
        format!(
            "coding_eval_improvement schema={} baseline={:.3} candidate={:.3} delta={:.3} baseline_pass={:.3} candidate_pass={:.3} profile_coverage_delta={} overfit_suspected={} behavior_improved={} reasons={}",
            self.schema_version,
            self.baseline_score,
            self.candidate_score,
            self.score_delta,
            self.baseline_pass_rate,
            self.candidate_pass_rate,
            self.profile_coverage_delta,
            self.overfit_suspected,
            self.behavior_improved,
            self.reasons.join("|")
        )
    }
}

pub fn default_coding_eval_corpus() -> CodingEvalCorpus {
    CodingEvalCorpus {
        fixtures: default_coding_eval_fixtures(),
        scoring_profiles: CodingEvalProfileKind::expected_profiles()
            .into_iter()
            .map(CodingEvalScoringProfile::for_kind)
            .collect(),
    }
}

pub fn coding_eval_corpus_from_fixture_tsv(input: &str) -> Result<CodingEvalCorpus, Vec<String>> {
    let mut failures = Vec::new();
    let mut fixtures = Vec::new();

    for (line_index, raw_line) in input.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let fields = line.split('\t').map(str::trim).collect::<Vec<_>>();
        if fields.len() != 5 {
            failures.push(format!("line{}:expected_5_tsv_fields", line_index + 1));
            continue;
        }

        let Some(profile) = parse_coding_eval_profile(fields[1]) else {
            failures.push(format!(
                "line{}:unknown_profile:{}",
                line_index + 1,
                fields[1]
            ));
            continue;
        };
        let expected_markers = fields[3]
            .split('|')
            .map(str::trim)
            .filter(|marker| !marker.is_empty())
            .collect::<Vec<_>>();
        if expected_markers.is_empty() {
            failures.push(format!("line{}:missing_expected_markers", line_index + 1));
            continue;
        }

        let validation_plans = match fields[4] {
            "none" | "" => Vec::new(),
            "cargo" => rust_fixture_validation_plans(fields[0]),
            other => {
                failures.push(format!(
                    "line{}:unknown_validation:{}",
                    line_index + 1,
                    other
                ));
                continue;
            }
        };
        fixtures.push(
            CodingEvalFixture::new(fields[0], profile, fields[2], expected_markers)
                .with_validation_plans(validation_plans),
        );
    }

    let corpus = CodingEvalCorpus {
        fixtures,
        scoring_profiles: CodingEvalProfileKind::expected_profiles()
            .into_iter()
            .map(CodingEvalScoringProfile::for_kind)
            .collect(),
    };
    failures.extend(corpus.validate().failures);
    if failures.is_empty() {
        Ok(corpus)
    } else {
        Err(failures)
    }
}

pub fn default_coding_eval_fixtures() -> Vec<CodingEvalFixture> {
    vec![
        CodingEvalFixture::new(
            "english-instruction-error-handling",
            CodingEvalProfileKind::EnglishInstruction,
            "Explain how to handle a recoverable Rust parsing error without panicking. Include a concise example and a validation step.",
            ["Result", "error context", "validation", "no panic"],
        ),
        CodingEvalFixture::new(
            "chinese-instruction-borrowing",
            CodingEvalProfileKind::ChineseInstruction,
            "用中文解释 Rust 借用检查器如何帮助避免数据竞争，并给出一个简短的修改建议。",
            ["借用", "数据竞争", "所有权", "修改建议"],
        ),
        CodingEvalFixture::new(
            "rust-codegen-parse-port",
            CodingEvalProfileKind::RustCodeGeneration,
            "Write a small Rust function parse_port(input: &str) -> Result<u16, String> that trims whitespace, rejects zero, and returns useful errors.",
            ["fn parse_port", "Result<u16", "trim", "parse", "zero"],
        )
        .with_validation_plans(rust_fixture_validation_plans("rust-codegen-parse-port")),
        CodingEvalFixture::new(
            "rust-repair-borrowed-prefix",
            CodingEvalProfileKind::RustRepair,
            "Repair a Rust helper so it returns a borrowed prefix when possible instead of cloning every string. Explain why the lifetime is valid.",
            ["Cow", "Borrowed", "Owned", "lifetime", "cargo test"],
        )
        .with_validation_plans(rust_fixture_validation_plans("rust-repair-borrowed-prefix")),
        CodingEvalFixture::new(
            "multilingual-coding-explain-result",
            CodingEvalProfileKind::MultilingualCodingExplanation,
            "Explain in English and Chinese why a Rust service should return Result instead of unwrap in request handling.",
            ["Result", "unwrap", "错误处理", "请求处理"],
        ),
    ]
}

fn parse_coding_eval_profile(value: &str) -> Option<CodingEvalProfileKind> {
    CodingEvalProfileKind::expected_profiles()
        .into_iter()
        .find(|profile| profile.as_str() == value)
}

pub fn sample_passing_observations(corpus: &CodingEvalCorpus) -> Vec<CodingEvalObservation> {
    corpus
        .fixtures
        .iter()
        .map(|fixture| {
            let output = sample_output_for_fixture(fixture);
            CodingEvalObservation::new(fixture.id.clone(), output)
                .with_compile(
                    fixture.requires_cargo_check(),
                    fixture.requires_cargo_check(),
                )
                .with_unit_tests(fixture.requires_cargo_test(), fixture.requires_cargo_test())
                .with_runtime_metrics(0.72, 480, 1_200)
                .with_benchmark_regression(0.0)
                .with_redaction(true)
        })
        .collect()
}

fn score_fixture(
    fixture: &CodingEvalFixture,
    profile: &CodingEvalScoringProfile,
    observation: &CodingEvalObservation,
) -> CodingEvalResult {
    let marker_coverage = marker_coverage(&observation.output, &fixture.expected_markers);
    let mut failures = Vec::new();

    if marker_coverage < profile.thresholds.min_marker_coverage {
        push_category(
            &mut failures,
            CodingEvalFailureCategory::MissingExpectedMarker,
        );
    }
    if fixture
        .forbidden_markers
        .iter()
        .any(|marker| contains_marker(&observation.output, marker))
    {
        push_category(&mut failures, CodingEvalFailureCategory::ForbiddenMarker);
    }
    if profile.thresholds.require_compile_check {
        if !observation.compile_checked {
            push_category(
                &mut failures,
                CodingEvalFailureCategory::CompileCheckMissing,
            );
        } else if !observation.compile_passed {
            push_category(&mut failures, CodingEvalFailureCategory::CompileCheckFailed);
        }
    }
    if profile.thresholds.require_unit_tests {
        if !observation.unit_test_checked {
            push_category(&mut failures, CodingEvalFailureCategory::UnitTestMissing);
        } else if !observation.unit_test_passed {
            push_category(&mut failures, CodingEvalFailureCategory::UnitTestFailed);
        }
    }
    if observation.benchmark_regression_pct > profile.thresholds.max_benchmark_regression_pct {
        push_category(
            &mut failures,
            CodingEvalFailureCategory::BenchmarkRegression,
        );
    }
    if observation.memory_hit_rate < profile.thresholds.min_memory_hit_rate {
        push_category(&mut failures, CodingEvalFailureCategory::MemoryMiss);
    }
    if observation.tokens > profile.thresholds.max_tokens {
        push_category(
            &mut failures,
            CodingEvalFailureCategory::TokenBudgetExceeded,
        );
    }
    if observation.latency_ms > profile.thresholds.max_latency_ms {
        push_category(
            &mut failures,
            CodingEvalFailureCategory::LatencyBudgetExceeded,
        );
    }
    if profile.thresholds.require_redaction
        && (!observation.redaction_passed
            || contains_private_or_unlicensed_marker(&observation.output))
    {
        push_category(&mut failures, CodingEvalFailureCategory::RedactionViolation);
    }
    if observation.overfit_signature_seen
        || fixture
            .overfit_guard_markers
            .iter()
            .any(|marker| contains_marker(&observation.output, marker))
    {
        push_category(&mut failures, CodingEvalFailureCategory::OverfitRisk);
    }
    if !fixture.provenance_safe() {
        push_category(
            &mut failures,
            CodingEvalFailureCategory::UnsafeFixtureProvenance,
        );
    }

    let score = score_from_failures(marker_coverage, &failures);
    let passed = failures.is_empty() && score >= 0.72;
    let evidence_packet = CodingEvalEvidencePacket {
        schema_version: CODING_EVAL_SCHEMA_VERSION,
        fixture_id: fixture.id.clone(),
        profile: fixture.profile,
        prompt_digest: fixture.prompt_digest(),
        output_digest: stable_redaction_digest([
            "coding-eval-output",
            fixture.id.as_str(),
            observation.output.as_str(),
        ]),
        validation_commands: fixture.validation_command_lines(),
        marker_coverage,
        compile_checked: observation.compile_checked,
        compile_passed: observation.compile_passed,
        unit_test_checked: observation.unit_test_checked,
        unit_test_passed: observation.unit_test_passed,
        benchmark_regression_pct: observation.benchmark_regression_pct,
        memory_hit_rate: observation.memory_hit_rate,
        tokens: observation.tokens,
        latency_ms: observation.latency_ms,
        redaction_passed: observation.redaction_passed,
        overfit_signature_seen: observation.overfit_signature_seen,
        failure_categories: failures.clone(),
        provenance_digest: stable_redaction_digest([
            "coding-eval-provenance",
            fixture.provenance.as_str(),
        ]),
    };

    CodingEvalResult {
        fixture_id: fixture.id.clone(),
        profile: fixture.profile,
        marker_coverage,
        score,
        passed,
        failure_categories: failures,
        evidence_packet,
    }
}

fn rust_fixture_validation_plans(fixture_id: &str) -> Vec<VerificationPlan> {
    let manifest_path = format!("state/eval-fixtures/{fixture_id}/Cargo.toml");
    vec![
        VerificationPlan::cargo_check_manifest(&manifest_path),
        VerificationPlan::cargo_test_manifest(&manifest_path),
    ]
}

fn sample_output_for_fixture(fixture: &CodingEvalFixture) -> String {
    match fixture.profile {
        CodingEvalProfileKind::EnglishInstruction => {
            "Use Result with error context, return the error instead of no panic, and add a validation step.".to_owned()
        }
        CodingEvalProfileKind::ChineseInstruction => {
            "借用 和 所有权 让 Rust 在编译期避免 数据竞争；修改建议 是缩小可变借用作用域。".to_owned()
        }
        CodingEvalProfileKind::RustCodeGeneration => {
            "fn parse_port(input: &str) -> Result<u16, String> { let value: u16 = input.trim().parse().map_err(|_| \"parse error\".to_owned())?; if value == 0 { return Err(\"zero port\".to_owned()); } Ok(value) }".to_owned()
        }
        CodingEvalProfileKind::RustRepair => {
            "Use std::borrow::Cow and return Cow::Borrowed for the prefix or Cow::Owned when allocation is needed; cargo test proves the lifetime remains tied to the input.".to_owned()
        }
        CodingEvalProfileKind::MultilingualCodingExplanation => {
            "Return Result instead of unwrap so request handling can report errors; 中文: Result 让 错误处理 可恢复，避免 unwrap 导致 请求处理 崩溃。".to_owned()
        }
    }
}

fn plan_contains_cargo_check(plan: &VerificationPlan) -> bool {
    plan.commands
        .iter()
        .any(|command| command.program == "cargo" && command.args.iter().any(|arg| arg == "check"))
}

fn plan_contains_cargo_test(plan: &VerificationPlan) -> bool {
    plan.commands
        .iter()
        .any(|command| command.program == "cargo" && command.args.iter().any(|arg| arg == "test"))
}

fn marker_coverage(output: &str, expected_markers: &[String]) -> f32 {
    if expected_markers.is_empty() {
        return 0.0;
    }
    let matched = expected_markers
        .iter()
        .filter(|marker| contains_marker(output, marker))
        .count();
    matched as f32 / expected_markers.len() as f32
}

fn contains_marker(value: &str, marker: &str) -> bool {
    let value_lower = value.to_ascii_lowercase();
    let marker_lower = marker.to_ascii_lowercase();
    value.contains(marker) || value_lower.contains(&marker_lower)
}

fn contains_private_or_unlicensed_marker(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "password",
        "secret",
        "api_key",
        "private prompt",
        "raw conversation",
        "tenant:",
        "hidden reasoning",
        "chain-of-thought",
        "unreviewed external source",
        "proprietary benchmark",
        "leetcode",
        "humaneval",
        "mbpp",
        "curl ",
        "powershell -enc",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn default_forbidden_markers() -> Vec<String> {
    vec![
        "unwrap without explanation".to_owned(),
        "panic is fine".to_owned(),
        "ignore validation".to_owned(),
        "private prompt".to_owned(),
        "secret".to_owned(),
        "unreviewed external source".to_owned(),
    ]
}

fn score_from_failures(marker_coverage: f32, failures: &[CodingEvalFailureCategory]) -> f32 {
    let mut score = marker_coverage.clamp(0.0, 1.0);
    for failure in failures {
        score -= match failure {
            CodingEvalFailureCategory::MissingExpectedMarker => 0.18,
            CodingEvalFailureCategory::ForbiddenMarker => 0.20,
            CodingEvalFailureCategory::CompileCheckMissing => 0.16,
            CodingEvalFailureCategory::CompileCheckFailed => 0.25,
            CodingEvalFailureCategory::UnitTestMissing => 0.14,
            CodingEvalFailureCategory::UnitTestFailed => 0.22,
            CodingEvalFailureCategory::BenchmarkRegression => 0.12,
            CodingEvalFailureCategory::MemoryMiss => 0.08,
            CodingEvalFailureCategory::TokenBudgetExceeded => 0.08,
            CodingEvalFailureCategory::LatencyBudgetExceeded => 0.08,
            CodingEvalFailureCategory::RedactionViolation => 0.30,
            CodingEvalFailureCategory::OverfitRisk => 0.30,
            CodingEvalFailureCategory::UnsafeFixtureProvenance => 0.35,
        };
    }
    score.clamp(0.0, 1.0)
}

fn push_category(
    failures: &mut Vec<CodingEvalFailureCategory>,
    category: CodingEvalFailureCategory,
) {
    if !failures.contains(&category) {
        failures.push(category);
    }
}

fn ratio(numerator: usize, denominator: usize) -> f32 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f32 / denominator as f32
    }
}

fn finite_unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn finite_nonnegative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn stable_redaction_digest<'a>(parts: impl IntoIterator<Item = &'a str>) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for part in parts {
        for byte in part.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash ^= 0xff;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("redaction-digest:{hash:016x}")
}

fn bool_field(value: bool) -> &'static str {
    if value { "1" } else { "0" }
}

fn escape_field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('|', "\\p")
}

fn map_summary(map: &BTreeMap<String, usize>) -> String {
    map.iter()
        .map(|(key, value)| format!("{key}:{value}"))
        .collect::<Vec<_>>()
        .join("|")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coding_eval_corpus_loads_all_multilingual_profiles() {
        let corpus = default_coding_eval_corpus();
        let report = corpus.validate();

        assert!(report.passed(), "{:?}", report.failures);
        assert_eq!(report.fixture_count, 5);
        assert_eq!(report.scoring_profile_count, 5);
        for profile in CodingEvalProfileKind::expected_profiles() {
            assert!(
                corpus
                    .fixtures
                    .iter()
                    .any(|fixture| fixture.profile == profile)
            );
            assert!(corpus.profile(profile).is_some());
        }
        assert!(
            corpus
                .fixtures
                .iter()
                .filter(|fixture| matches!(
                    fixture.profile,
                    CodingEvalProfileKind::RustCodeGeneration | CodingEvalProfileKind::RustRepair
                ))
                .all(|fixture| fixture.requires_cargo_check() && fixture.requires_cargo_test())
        );
    }

    #[test]
    fn coding_eval_scorer_serializes_redacted_packets() {
        let corpus = default_coding_eval_corpus();
        let report = corpus.score_observations(&sample_passing_observations(&corpus));

        assert_eq!(report.result_count, 5);
        assert_eq!(report.passed_count, 5);
        assert!(report.evidence_is_redacted());
        assert!(report.summary_line().contains("coding_eval_suite"));
        assert!(
            report
                .evidence_packets
                .iter()
                .all(|packet| packet.record_line().contains("redaction-digest:"))
        );
        assert!(
            report
                .evidence_packets
                .iter()
                .all(|packet| !packet.record_line().contains("Explain how"))
        );
    }

    #[test]
    fn coding_eval_thresholds_fail_missing_compile_and_tests() {
        let corpus = default_coding_eval_corpus();
        let fixture = corpus
            .fixtures
            .iter()
            .find(|fixture| fixture.profile == CodingEvalProfileKind::RustRepair)
            .expect("rust repair fixture");
        let observation = CodingEvalObservation::new(
            fixture.id.clone(),
            "Cow Borrowed Owned lifetime cargo test",
        )
        .with_runtime_metrics(0.7, 500, 1_000);

        let result = corpus
            .score_observation(&observation)
            .expect("score observation");

        assert!(!result.passed);
        assert!(
            result
                .failure_categories
                .contains(&CodingEvalFailureCategory::CompileCheckMissing)
        );
        assert!(
            result
                .failure_categories
                .contains(&CodingEvalFailureCategory::UnitTestMissing)
        );
    }

    #[test]
    fn coding_eval_redaction_blocks_private_or_unlicensed_payloads() {
        let corpus = default_coding_eval_corpus();
        let fixture = &corpus.fixtures[0];
        let observation = CodingEvalObservation::new(
            fixture.id.clone(),
            "Result validation no panic error context plus private prompt password=secret",
        )
        .with_runtime_metrics(0.7, 400, 1_000)
        .with_redaction(false);

        let result = corpus
            .score_observation(&observation)
            .expect("score observation");

        assert!(!result.passed);
        assert!(
            result
                .failure_categories
                .contains(&CodingEvalFailureCategory::RedactionViolation)
        );
        assert!(
            result
                .evidence_packet
                .record_line()
                .contains("redaction-digest:")
        );
        assert!(
            !result
                .evidence_packet
                .record_line()
                .contains("password=secret")
        );
    }

    #[test]
    fn coding_eval_improvement_distinguishes_overfit_from_improvement() {
        let corpus = default_coding_eval_corpus();
        let baseline_observations = corpus
            .fixtures
            .iter()
            .map(|fixture| {
                CodingEvalObservation::new(fixture.id.clone(), "partial answer")
                    .with_runtime_metrics(0.1, 300, 900)
                    .with_redaction(true)
            })
            .collect::<Vec<_>>();
        let baseline = corpus.score_observations(&baseline_observations);
        let candidate = corpus.score_observations(&sample_passing_observations(&corpus));
        let improvement = CodingEvalImprovementReport::compare(&baseline, &candidate);

        assert!(improvement.behavior_improved, "{improvement:?}");
        assert!(!improvement.overfit_suspected);

        let overfit_observations = corpus
            .fixtures
            .iter()
            .map(|fixture| {
                CodingEvalObservation::new(
                    fixture.id.clone(),
                    format!(
                        "{} fixture-id hard-coded answer",
                        sample_output_for_fixture(fixture)
                    ),
                )
                .with_compile(
                    fixture.requires_cargo_check(),
                    fixture.requires_cargo_check(),
                )
                .with_unit_tests(fixture.requires_cargo_test(), fixture.requires_cargo_test())
                .with_runtime_metrics(0.8, 400, 900)
                .with_overfit_signature()
            })
            .collect::<Vec<_>>();
        let overfit = corpus.score_observations(&overfit_observations);
        let overfit_report = CodingEvalImprovementReport::compare(&baseline, &overfit);

        assert!(overfit_report.overfit_suspected);
        assert!(!overfit_report.behavior_improved);
    }
}
