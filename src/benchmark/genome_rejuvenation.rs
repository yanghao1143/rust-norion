use std::collections::BTreeSet;

use crate::hierarchy::TaskProfile;
use crate::privacy_redaction::{contains_private_or_executable_marker, stable_redaction_digest};
use crate::reasoning_genome::{
    GeneLifecycleAction, GeneScissorsIntent, GeneValidationStatus, GenomeExpression,
    GenomeExpressionInput, MutationPlan, ReasoningGene, ReasoningGeneKind, ReasoningGenome,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GenomeRejuvenationDecisionKind {
    Keep,
    Relabel,
    Refresh,
    Regenerate,
    Quarantine,
    Tombstone,
}

impl GenomeRejuvenationDecisionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Keep => "keep",
            Self::Relabel => "relabel",
            Self::Refresh => "refresh",
            Self::Regenerate => "regenerate",
            Self::Quarantine => "quarantine",
            Self::Tombstone => "tombstone",
        }
    }

    pub fn required_coverage() -> [Self; 6] {
        [
            Self::Keep,
            Self::Relabel,
            Self::Refresh,
            Self::Regenerate,
            Self::Quarantine,
            Self::Tombstone,
        ]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenomeRejuvenationCase {
    pub id: String,
    pub profile: TaskProfile,
    pub gene: ReasoningGene,
    pub task_repetitions: usize,
    pub expression_input: GenomeExpressionInput,
    pub baseline_routing_cost_proxy: usize,
    pub baseline_memory_hit_usefulness: f32,
    pub expected_decisions: Vec<GenomeRejuvenationDecisionKind>,
}

impl GenomeRejuvenationCase {
    pub fn new(
        id: impl Into<String>,
        profile: TaskProfile,
        gene: ReasoningGene,
        expression_input: GenomeExpressionInput,
        expected_decisions: Vec<GenomeRejuvenationDecisionKind>,
    ) -> Self {
        Self {
            id: id.into(),
            profile,
            gene,
            task_repetitions: 3,
            expression_input,
            baseline_routing_cost_proxy: 120,
            baseline_memory_hit_usefulness: 0.70,
            expected_decisions,
        }
    }

    pub fn with_task_repetitions(mut self, task_repetitions: usize) -> Self {
        self.task_repetitions = task_repetitions.max(1);
        self
    }

    pub fn with_baseline(mut self, routing_cost_proxy: usize, memory_hit_usefulness: f32) -> Self {
        self.baseline_routing_cost_proxy = routing_cost_proxy.max(1);
        self.baseline_memory_hit_usefulness = clamp_unit(memory_hit_usefulness);
        self
    }

    fn stable_anchor_id(&self) -> String {
        format!("genome:rejuvenation:{}:stable", slug(&self.id))
    }

    fn case_digest(&self) -> String {
        digest([
            self.id.as_str(),
            self.gene.id.as_str(),
            self.gene.kind.as_str(),
            self.gene.status.as_str(),
        ])
    }

    fn ledger_input_digest(&self) -> String {
        let age = self.gene.age.to_string();
        let fitness = format!("{:.3}", self.gene.fitness);
        let drift = format!("{:.3}", self.gene.drift_score);
        let repetitions = self.task_repetitions.to_string();
        let quality = format!("{:.3}", self.expression_input.quality);
        let reward = format!("{:.3}", self.expression_input.process_reward);
        let contradictions = self.expression_input.contradiction_count.to_string();
        let critical = self
            .expression_input
            .critical_reflection_issue_count
            .to_string();

        digest([
            self.id.as_str(),
            self.gene.kind.as_str(),
            age.as_str(),
            fitness.as_str(),
            drift.as_str(),
            repetitions.as_str(),
            quality.as_str(),
            reward.as_str(),
            contradictions.as_str(),
            critical.as_str(),
        ])
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenomeRejuvenationSnapshot {
    pub gene_count: usize,
    pub average_fitness: f32,
    pub average_drift: f32,
    pub average_decay: f32,
    pub aged_gene_count: usize,
    pub malignant_gene_count: usize,
    pub routing_cost_proxy: usize,
    pub wasted_compute_proxy: usize,
    pub memory_hit_usefulness: f32,
    pub validation_status: GeneValidationStatus,
    pub rollback_anchor_id: String,
    pub replay_digest: String,
}

impl GenomeRejuvenationSnapshot {
    pub fn fitness_delta(&self, before: &Self) -> f32 {
        self.average_fitness - before.average_fitness
    }

    pub fn wasted_compute_reduction(&self, before: &Self) -> usize {
        before
            .wasted_compute_proxy
            .saturating_sub(self.wasted_compute_proxy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenomeRejuvenationDecision {
    pub kind: GenomeRejuvenationDecisionKind,
    pub gene_digest: String,
    pub plan_digest: Option<String>,
    pub reason: String,
    pub validation_status: GeneValidationStatus,
    pub rollback_anchor_id: String,
    pub replay_digest: String,
    pub preview_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
    pub approval_required: bool,
}

impl GenomeRejuvenationDecision {
    pub fn is_safe_preview(&self) -> bool {
        self.preview_only
            && !self.write_allowed
            && !self.applied
            && !self.rollback_anchor_id.trim().is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenomeRejuvenationCaseResult {
    pub case_digest: String,
    pub ledger_input_digest: String,
    pub profile: TaskProfile,
    pub target_gene_digest: String,
    pub task_repetitions: usize,
    pub before: GenomeRejuvenationSnapshot,
    pub after: GenomeRejuvenationSnapshot,
    pub decisions: Vec<GenomeRejuvenationDecision>,
    pub expression_read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
    pub rollback_ready: bool,
    pub failures: Vec<String>,
}

impl GenomeRejuvenationCaseResult {
    pub fn decision_kinds(&self) -> Vec<GenomeRejuvenationDecisionKind> {
        let mut kinds = Vec::new();
        for decision in &self.decisions {
            if !kinds.contains(&decision.kind) {
                kinds.push(decision.kind);
            }
        }
        kinds
    }

    pub fn decision_summary(&self) -> String {
        self.decision_kinds()
            .iter()
            .map(|kind| kind.as_str())
            .collect::<Vec<_>>()
            .join("|")
    }

    pub fn wasted_compute_reduction(&self) -> usize {
        self.after.wasted_compute_reduction(&self.before)
    }

    pub fn memory_usefulness_delta(&self) -> f32 {
        self.after.memory_hit_usefulness - self.before.memory_hit_usefulness
    }

    pub fn ledger_line(&self) -> String {
        format!(
            "genome_rejuvenation_v1 case={} ledger_input={} profile={:?} target={} repetitions={} decisions={} before_fitness={:.3} after_fitness={:.3} before_drift={:.3} after_drift={:.3} before_wasted={} after_wasted={} wasted_reduction={} before_memory_usefulness={:.3} after_memory_usefulness={:.3} validation={} rollback_ready={} read_only={} write_allowed={} applied={} replay_digest={}",
            self.case_digest,
            self.ledger_input_digest,
            self.profile,
            self.target_gene_digest,
            self.task_repetitions,
            self.decision_summary(),
            self.before.average_fitness,
            self.after.average_fitness,
            self.before.average_drift,
            self.after.average_drift,
            self.before.wasted_compute_proxy,
            self.after.wasted_compute_proxy,
            self.wasted_compute_reduction(),
            self.before.memory_hit_usefulness,
            self.after.memory_hit_usefulness,
            self.after.validation_status.as_str(),
            self.rollback_ready,
            self.expression_read_only,
            self.write_allowed,
            self.applied,
            self.after.replay_digest
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct GenomeRejuvenationSimulationReport {
    pub results: Vec<GenomeRejuvenationCaseResult>,
    pub failures: Vec<String>,
}

impl GenomeRejuvenationSimulationReport {
    pub fn case_count(&self) -> usize {
        self.results.len()
    }

    pub fn decision_count(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.decisions.len())
            .sum()
    }

    pub fn covered_decision_kinds(&self) -> Vec<GenomeRejuvenationDecisionKind> {
        self.results
            .iter()
            .flat_map(GenomeRejuvenationCaseResult::decision_kinds)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn count_decision(&self, kind: GenomeRejuvenationDecisionKind) -> usize {
        self.results
            .iter()
            .flat_map(|result| result.decisions.iter())
            .filter(|decision| decision.kind == kind)
            .count()
    }

    pub fn rollback_ready_count(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.rollback_ready)
            .count()
    }

    pub fn write_allowed_count(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.write_allowed)
            .count()
    }

    pub fn applied_count(&self) -> usize {
        self.results.iter().filter(|result| result.applied).count()
    }

    pub fn replay_digest_count(&self) -> usize {
        self.results
            .iter()
            .filter(|result| {
                result.ledger_input_digest.starts_with("redaction-digest:")
                    && result.after.replay_digest.starts_with("redaction-digest:")
            })
            .count()
    }

    pub fn total_wasted_compute_before(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.before.wasted_compute_proxy)
            .sum()
    }

    pub fn total_wasted_compute_after(&self) -> usize {
        self.results
            .iter()
            .map(|result| result.after.wasted_compute_proxy)
            .sum()
    }

    pub fn total_wasted_compute_reduction(&self) -> usize {
        self.total_wasted_compute_before()
            .saturating_sub(self.total_wasted_compute_after())
    }

    pub fn average_memory_usefulness_delta(&self) -> f32 {
        average(
            self.results.iter().map(|result| {
                result.after.memory_hit_usefulness - result.before.memory_hit_usefulness
            }),
        )
    }

    pub fn ledger_lines(&self) -> Vec<String> {
        self.results
            .iter()
            .map(GenomeRejuvenationCaseResult::ledger_line)
            .collect()
    }

    pub fn ledger_is_digest_only(&self) -> bool {
        self.ledger_lines().iter().all(|line| {
            !contains_private_or_executable_marker(line)
                && !line.contains("prompt")
                && !line.contains("answer")
                && !line.contains("secret")
                && !line.contains("malicious")
        })
    }

    pub fn summary_line(&self) -> String {
        let covered = self
            .covered_decision_kinds()
            .iter()
            .map(|kind| kind.as_str())
            .collect::<Vec<_>>()
            .join("|");
        format!(
            "genome_rejuvenation cases={} decisions={} covered={} wasted_before={} wasted_after={} wasted_reduction={} avg_memory_usefulness_delta={:.3} rollback_ready={} replay_digests={} write_allowed={} applied={} failures={}",
            self.case_count(),
            self.decision_count(),
            covered,
            self.total_wasted_compute_before(),
            self.total_wasted_compute_after(),
            self.total_wasted_compute_reduction(),
            self.average_memory_usefulness_delta(),
            self.rollback_ready_count(),
            self.replay_digest_count(),
            self.write_allowed_count(),
            self.applied_count(),
            self.failures.len()
        )
    }

    pub fn evaluate(
        &self,
        gate: &GenomeRejuvenationSimulationGate,
    ) -> GenomeRejuvenationSimulationGateReport {
        let mut failures = self.failures.clone();
        require_at_least(&mut failures, "cases", self.case_count(), gate.min_cases);
        require_at_least(
            &mut failures,
            "decisions",
            self.decision_count(),
            gate.min_decisions,
        );
        require_at_least(
            &mut failures,
            "rollback_ready",
            self.rollback_ready_count(),
            gate.min_rollback_ready,
        );
        require_at_least(
            &mut failures,
            "replay_digests",
            self.replay_digest_count(),
            gate.min_replay_digests,
        );
        require_at_least(
            &mut failures,
            "wasted_compute_reduction",
            self.total_wasted_compute_reduction(),
            gate.min_wasted_compute_reduction,
        );

        if gate.require_all_decision_kinds {
            let covered = self.covered_decision_kinds();
            for kind in GenomeRejuvenationDecisionKind::required_coverage() {
                if !covered.contains(&kind) {
                    failures.push(format!("decision_kind_missing:{}", kind.as_str()));
                }
            }
        }
        if self.write_allowed_count() > gate.max_write_allowed {
            failures.push(format!(
                "write_allowed {} exceeds max {}",
                self.write_allowed_count(),
                gate.max_write_allowed
            ));
        }
        if self.applied_count() > gate.max_applied {
            failures.push(format!(
                "applied {} exceeds max {}",
                self.applied_count(),
                gate.max_applied
            ));
        }
        if gate.require_non_decreasing_memory_usefulness
            && self.results.iter().any(|result| {
                result.after.memory_hit_usefulness < result.before.memory_hit_usefulness
            })
        {
            failures.push("memory_usefulness_regressed".to_owned());
        }
        if gate.require_digest_only_ledger && !self.ledger_is_digest_only() {
            failures.push("ledger_not_digest_only".to_owned());
        }
        if failures.len() > gate.max_failures {
            failures.push(format!(
                "failures {} exceeds max {}",
                failures.len(),
                gate.max_failures
            ));
        }

        GenomeRejuvenationSimulationGateReport {
            passed: failures.is_empty(),
            failures,
        }
    }

    fn validate(mut self, cases: &[GenomeRejuvenationCase]) -> Self {
        if cases.is_empty() {
            self.failures
                .push("genome_rejuvenation_cases_missing".to_owned());
        }
        for (case, result) in cases.iter().zip(&self.results) {
            validate_result(case, result, &mut self.failures);
        }
        if !self.ledger_is_digest_only() {
            self.failures
                .push("genome_rejuvenation_ledger_not_digest_only".to_owned());
        }
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GenomeRejuvenationSimulationGate {
    pub min_cases: usize,
    pub min_decisions: usize,
    pub require_all_decision_kinds: bool,
    pub min_rollback_ready: usize,
    pub min_replay_digests: usize,
    pub min_wasted_compute_reduction: usize,
    pub max_write_allowed: usize,
    pub max_applied: usize,
    pub require_non_decreasing_memory_usefulness: bool,
    pub require_digest_only_ledger: bool,
    pub max_failures: usize,
}

impl Default for GenomeRejuvenationSimulationGate {
    fn default() -> Self {
        Self {
            min_cases: 4,
            min_decisions: 6,
            require_all_decision_kinds: true,
            min_rollback_ready: 4,
            min_replay_digests: 4,
            min_wasted_compute_reduction: 1,
            max_write_allowed: 0,
            max_applied: 0,
            require_non_decreasing_memory_usefulness: true,
            require_digest_only_ledger: true,
            max_failures: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenomeRejuvenationSimulationGateReport {
    pub passed: bool,
    pub failures: Vec<String>,
}

impl GenomeRejuvenationSimulationGateReport {
    pub fn summary_line(&self) -> String {
        format!(
            "genome_rejuvenation_gate passed={} failures={}",
            self.passed,
            self.failures.len()
        )
    }
}

pub fn run_default_genome_rejuvenation_simulation() -> GenomeRejuvenationSimulationReport {
    run_genome_rejuvenation_simulation(&default_genome_rejuvenation_cases())
}

pub fn run_genome_rejuvenation_simulation(
    cases: &[GenomeRejuvenationCase],
) -> GenomeRejuvenationSimulationReport {
    let results = cases.iter().map(run_case).collect::<Vec<_>>();
    GenomeRejuvenationSimulationReport {
        results,
        failures: Vec::new(),
    }
    .validate(cases)
}

pub fn default_genome_rejuvenation_cases() -> Vec<GenomeRejuvenationCase> {
    vec![
        GenomeRejuvenationCase::new(
            "stable-keep",
            TaskProfile::Coding,
            ReasoningGene::new(
                "gene:rejuvenation:stable-retrieval",
                ReasoningGeneKind::Retrieval,
                "stable retrieval",
                "reuse approved digest-only memory evidence",
            )
            .with_tags(["memory", "approved", "keep"])
            .with_health(2, 0.92, 0.04),
            healthy_input(),
            vec![GenomeRejuvenationDecisionKind::Keep],
        )
        .with_task_repetitions(3)
        .with_baseline(118, 0.82),
        GenomeRejuvenationCase::new(
            "stale-label-relabel",
            TaskProfile::Coding,
            ReasoningGene::new(
                "gene:rejuvenation:stale-label",
                ReasoningGeneKind::Language,
                "",
                "",
            )
            .with_tags(["language", "stale"])
            .with_health(13, 0.66, 0.12),
            stale_input(),
            vec![GenomeRejuvenationDecisionKind::Relabel],
        )
        .with_task_repetitions(7)
        .with_baseline(168, 0.54),
        GenomeRejuvenationCase::new(
            "low-fitness-refresh",
            TaskProfile::Coding,
            ReasoningGene::new(
                "gene:rejuvenation:low-fitness",
                ReasoningGeneKind::Routing,
                "low-fitness route scorer",
                "route threshold evidence has drifted and needs refreshed validation",
            )
            .with_tags(["routing", "refresh"])
            .with_health(9, 0.32, 0.28),
            refresh_input(),
            vec![GenomeRejuvenationDecisionKind::Refresh],
        )
        .with_task_repetitions(9)
        .with_baseline(196, 0.48),
        GenomeRejuvenationCase::new(
            "malignant-regeneration",
            TaskProfile::Coding,
            ReasoningGene::new(
                "gene:rejuvenation:malignant-safety",
                ReasoningGeneKind::Safety,
                "polluted safety guard",
                "unsafe safety behavior must be isolated before reuse",
            )
            .with_tags(["safety", "quarantine"])
            .with_health(12, 0.18, 0.91),
            malignant_input(),
            vec![
                GenomeRejuvenationDecisionKind::Quarantine,
                GenomeRejuvenationDecisionKind::Regenerate,
                GenomeRejuvenationDecisionKind::Tombstone,
            ],
        )
        .with_task_repetitions(5)
        .with_baseline(230, 0.26),
    ]
}

fn run_case(case: &GenomeRejuvenationCase) -> GenomeRejuvenationCaseResult {
    let genome = ReasoningGenome::new(
        format!("genome:rejuvenation:{}", slug(&case.id)),
        case.profile,
        case.stable_anchor_id(),
        vec![case.gene.clone()],
    );
    let before = before_snapshot(case, &genome);
    let expression = genome
        .with_feedback_health(&case.expression_input)
        .express(case.expression_input);
    let decisions = decisions_for_expression(case, &expression);
    let after = projected_after_snapshot(case, &before, &decisions, &expression);
    let rollback_ready = decisions.iter().all(|decision| {
        decision.is_safe_preview()
            && (decision.kind == GenomeRejuvenationDecisionKind::Keep || decision.approval_required)
    });
    let expression_read_only = expression.is_read_only_preview();
    let write_allowed =
        expression.write_allowed || decisions.iter().any(|decision| decision.write_allowed);
    let applied = expression.applied || decisions.iter().any(|decision| decision.applied);
    let mut failures = Vec::new();
    let result = GenomeRejuvenationCaseResult {
        case_digest: case.case_digest(),
        ledger_input_digest: case.ledger_input_digest(),
        profile: case.profile,
        target_gene_digest: digest([case.gene.id.as_str()]),
        task_repetitions: case.task_repetitions,
        before,
        after,
        decisions,
        expression_read_only,
        write_allowed,
        applied,
        rollback_ready,
        failures: Vec::new(),
    };
    validate_result(case, &result, &mut failures);
    GenomeRejuvenationCaseResult { failures, ..result }
}

fn decisions_for_expression(
    case: &GenomeRejuvenationCase,
    expression: &GenomeExpression,
) -> Vec<GenomeRejuvenationDecision> {
    let mut decisions = Vec::new();
    for record in expression
        .lifecycle_records
        .iter()
        .filter(|record| record.gene_id == case.gene.id)
    {
        let Some(kind) = decision_kind_for_action(case, record.action) else {
            continue;
        };
        let plan = matching_plan(expression, kind, &record.gene_id);
        let validation_status = plan
            .map(|plan| plan.validation_status)
            .unwrap_or(record.validation_status);
        let rollback_anchor_id = plan
            .map(|plan| plan.rollback_anchor_id.clone())
            .unwrap_or_else(|| record.rollback_anchor_id.clone());
        let plan_digest = plan.map(plan_digest);
        let replay_digest = digest([
            case.id.as_str(),
            record.gene_id.as_str(),
            kind.as_str(),
            validation_status.as_str(),
            rollback_anchor_id.as_str(),
        ]);
        decisions.push(GenomeRejuvenationDecision {
            kind,
            gene_digest: digest([record.gene_id.as_str()]),
            plan_digest,
            reason: sanitized_reason(plan, record.reason.as_str()),
            validation_status,
            rollback_anchor_id,
            replay_digest,
            preview_only: plan
                .map(MutationPlan::is_read_only_preview)
                .unwrap_or_else(|| record.is_read_only_preview()),
            write_allowed: plan
                .map(|plan| plan.admission_write_authorized)
                .unwrap_or(record.admission_write_authorized),
            applied: plan.map(|plan| plan.applied).unwrap_or(record.applied),
            approval_required: kind != GenomeRejuvenationDecisionKind::Keep,
        });
    }
    decisions
}

fn decision_kind_for_action(
    case: &GenomeRejuvenationCase,
    action: GeneLifecycleAction,
) -> Option<GenomeRejuvenationDecisionKind> {
    match action {
        GeneLifecycleAction::Keep => Some(GenomeRejuvenationDecisionKind::Keep),
        GeneLifecycleAction::Relabel
            if case.gene.label.trim().is_empty() || case.gene.purpose.trim().is_empty() =>
        {
            Some(GenomeRejuvenationDecisionKind::Relabel)
        }
        GeneLifecycleAction::Relabel => Some(GenomeRejuvenationDecisionKind::Refresh),
        GeneLifecycleAction::Quarantine => Some(GenomeRejuvenationDecisionKind::Quarantine),
        GeneLifecycleAction::Regenerate => Some(GenomeRejuvenationDecisionKind::Regenerate),
        GeneLifecycleAction::Cut => Some(GenomeRejuvenationDecisionKind::Tombstone),
        GeneLifecycleAction::Rollback => None,
    }
}

fn matching_plan<'a>(
    expression: &'a GenomeExpression,
    kind: GenomeRejuvenationDecisionKind,
    gene_id: &str,
) -> Option<&'a MutationPlan> {
    let intent = match kind {
        GenomeRejuvenationDecisionKind::Keep => return None,
        GenomeRejuvenationDecisionKind::Relabel | GenomeRejuvenationDecisionKind::Refresh => {
            GeneScissorsIntent::Relabel
        }
        GenomeRejuvenationDecisionKind::Regenerate => GeneScissorsIntent::Regenerate,
        GenomeRejuvenationDecisionKind::Quarantine => GeneScissorsIntent::Quarantine,
        GenomeRejuvenationDecisionKind::Tombstone => GeneScissorsIntent::Cut,
    };
    expression
        .mutation_plans
        .iter()
        .find(|plan| plan.target_gene_id == gene_id && plan.intent == intent)
}

fn before_snapshot(
    case: &GenomeRejuvenationCase,
    genome: &ReasoningGenome,
) -> GenomeRejuvenationSnapshot {
    let average_fitness = average(genome.genes.iter().map(|gene| gene.fitness));
    let average_drift = average(genome.genes.iter().map(|gene| gene.drift_score));
    let average_decay = average(genome.genes.iter().map(ReasoningGene::decay_score));
    let aged_gene_count = genome
        .genes
        .iter()
        .filter(|gene| gene.needs_relabel() && !gene.is_malignant())
        .count();
    let malignant_gene_count = genome
        .genes
        .iter()
        .filter(|gene| gene.is_malignant())
        .count();
    let missing_label_penalty =
        usize::from(case.gene.label.trim().is_empty() || case.gene.purpose.trim().is_empty()) * 36;
    let health_penalty =
        ((average_decay + average_drift + (1.0 - average_fitness)) * 95.0).round() as usize;
    let repeated_pressure = case.task_repetitions.saturating_mul(health_penalty / 3 + 1);
    let wasted_compute_proxy = missing_label_penalty
        .saturating_add(health_penalty)
        .saturating_add(repeated_pressure)
        .max(1);
    let routing_cost_proxy = case
        .baseline_routing_cost_proxy
        .saturating_add(wasted_compute_proxy);
    let memory_hit_usefulness = clamp_unit(
        case.baseline_memory_hit_usefulness
            - average_decay * 0.28
            - average_drift * 0.18
            - if missing_label_penalty > 0 { 0.08 } else { 0.0 },
    );
    let replay_digest = digest([
        case.id.as_str(),
        case.gene.id.as_str(),
        "before",
        &format!("{average_fitness:.3}"),
        &format!("{average_drift:.3}"),
        &wasted_compute_proxy.to_string(),
    ]);

    GenomeRejuvenationSnapshot {
        gene_count: genome.genes.len(),
        average_fitness,
        average_drift,
        average_decay,
        aged_gene_count,
        malignant_gene_count,
        routing_cost_proxy,
        wasted_compute_proxy,
        memory_hit_usefulness,
        validation_status: GeneValidationStatus::NotRequired,
        rollback_anchor_id: genome.stable_anchor_id.clone(),
        replay_digest,
    }
}

fn projected_after_snapshot(
    case: &GenomeRejuvenationCase,
    before: &GenomeRejuvenationSnapshot,
    decisions: &[GenomeRejuvenationDecision],
    expression: &GenomeExpression,
) -> GenomeRejuvenationSnapshot {
    let has = |kind| decisions.iter().any(|decision| decision.kind == kind);
    let (fitness_gain, drift_factor, waste_factor, usefulness_gain) =
        if has(GenomeRejuvenationDecisionKind::Regenerate)
            || has(GenomeRejuvenationDecisionKind::Quarantine)
            || has(GenomeRejuvenationDecisionKind::Tombstone)
        {
            (0.54, 0.22, 0.34, 0.42)
        } else if has(GenomeRejuvenationDecisionKind::Refresh) {
            (0.20, 0.55, 0.52, 0.18)
        } else if has(GenomeRejuvenationDecisionKind::Relabel) {
            (0.09, 0.70, 0.62, 0.14)
        } else {
            (0.01, 0.95, 0.94, 0.01)
        };
    let average_fitness = clamp_unit(before.average_fitness + fitness_gain);
    let average_drift = clamp_unit(before.average_drift * drift_factor);
    let average_decay = clamp_unit(before.average_decay * drift_factor);
    let wasted_compute_proxy = ((before.wasted_compute_proxy as f32) * waste_factor)
        .round()
        .max(0.0) as usize;
    let routing_cost_proxy = ((before.routing_cost_proxy as f32) * (0.82 + waste_factor * 0.10))
        .round()
        .max(1.0) as usize;
    let memory_hit_usefulness =
        clamp_unit(before.memory_hit_usefulness + usefulness_gain + fitness_gain * 0.10);
    let validation_status = if decisions
        .iter()
        .all(|decision| decision.kind == GenomeRejuvenationDecisionKind::Keep)
    {
        GeneValidationStatus::NotRequired
    } else {
        GeneValidationStatus::Pending
    };
    let replay_digest = digest([
        case.id.as_str(),
        "after",
        &decisions
            .iter()
            .map(|decision| decision.kind.as_str())
            .collect::<Vec<_>>()
            .join("|"),
        &format!("{average_fitness:.3}"),
        &format!("{average_drift:.3}"),
        &wasted_compute_proxy.to_string(),
    ]);

    GenomeRejuvenationSnapshot {
        gene_count: before.gene_count,
        average_fitness,
        average_drift,
        average_decay,
        aged_gene_count: 0,
        malignant_gene_count: 0,
        routing_cost_proxy,
        wasted_compute_proxy,
        memory_hit_usefulness,
        validation_status,
        rollback_anchor_id: expression.stable_anchor_id.clone(),
        replay_digest,
    }
}

fn validate_result(
    case: &GenomeRejuvenationCase,
    result: &GenomeRejuvenationCaseResult,
    failures: &mut Vec<String>,
) {
    if result.decisions.is_empty() {
        failures.push(format!("{}:decisions_missing", case.id));
    }
    let kinds = result.decision_kinds();
    for expected in &case.expected_decisions {
        if !kinds.contains(expected) {
            failures.push(format!(
                "{}:expected_decision_missing:{}",
                case.id,
                expected.as_str()
            ));
        }
    }
    if !all_finite([
        result.before.average_fitness,
        result.before.average_drift,
        result.before.average_decay,
        result.before.memory_hit_usefulness,
        result.after.average_fitness,
        result.after.average_drift,
        result.after.average_decay,
        result.after.memory_hit_usefulness,
    ]) {
        failures.push(format!("{}:non_finite_metrics", case.id));
    }
    if result.after.wasted_compute_proxy > result.before.wasted_compute_proxy {
        failures.push(format!("{}:wasted_compute_increased", case.id));
    }
    if result.after.memory_hit_usefulness < result.before.memory_hit_usefulness {
        failures.push(format!("{}:memory_usefulness_regressed", case.id));
    }
    if !result.expression_read_only || result.write_allowed || result.applied {
        failures.push(format!("{}:expression_not_preview_only", case.id));
    }
    if !result.rollback_ready {
        failures.push(format!("{}:rollback_not_ready", case.id));
    }
    for decision in &result.decisions {
        if !decision.is_safe_preview() {
            failures.push(format!(
                "{}:decision_not_safe_preview:{}",
                case.id,
                decision.kind.as_str()
            ));
        }
        if decision.kind != GenomeRejuvenationDecisionKind::Keep
            && decision.validation_status != GeneValidationStatus::Pending
        {
            failures.push(format!(
                "{}:decision_validation_not_pending:{}",
                case.id,
                decision.kind.as_str()
            ));
        }
        if !decision.replay_digest.starts_with("redaction-digest:") {
            failures.push(format!("{}:decision_replay_digest_missing", case.id));
        }
    }
    if !result.ledger_input_digest.starts_with("redaction-digest:")
        || !result.after.replay_digest.starts_with("redaction-digest:")
    {
        failures.push(format!("{}:replay_digest_missing", case.id));
    }
    if contains_private_or_executable_marker(&result.ledger_line()) {
        failures.push(format!("{}:ledger_contains_private_marker", case.id));
    }
}

fn sanitized_reason(plan: Option<&MutationPlan>, lifecycle_reason: &str) -> String {
    let reason = plan
        .map(|plan| plan.reason.as_str())
        .unwrap_or(lifecycle_reason);
    if contains_private_or_executable_marker(reason) {
        "redacted_reason".to_owned()
    } else {
        reason.to_owned()
    }
}

fn plan_digest(plan: &MutationPlan) -> String {
    digest([
        plan.id.as_str(),
        plan.intent.as_str(),
        plan.target_gene_id.as_str(),
        plan.rollback_anchor_id.as_str(),
        plan.validation_status.as_str(),
    ])
}

fn healthy_input() -> GenomeExpressionInput {
    GenomeExpressionInput {
        profile: TaskProfile::Coding,
        quality: 0.91,
        process_reward: 0.88,
        contradiction_count: 0,
        critical_reflection_issue_count: 0,
        revision_action_count: 0,
        used_memories: 3,
        memory_feedback_updates: 0,
        route_attention_fraction: 0.36,
        agent_team_collision_free: true,
        toolsmith_gate_passed: true,
        drift_memory_write_allowed: true,
        drift_rollback: false,
        runtime_kv_hold: false,
    }
}

fn stale_input() -> GenomeExpressionInput {
    GenomeExpressionInput {
        quality: 0.72,
        process_reward: 0.68,
        used_memories: 2,
        route_attention_fraction: 0.52,
        ..healthy_input()
    }
}

fn refresh_input() -> GenomeExpressionInput {
    GenomeExpressionInput {
        quality: 0.58,
        process_reward: 0.51,
        contradiction_count: 1,
        revision_action_count: 1,
        memory_feedback_updates: 1,
        route_attention_fraction: 0.78,
        drift_memory_write_allowed: false,
        ..healthy_input()
    }
}

fn malignant_input() -> GenomeExpressionInput {
    GenomeExpressionInput {
        quality: 0.36,
        process_reward: 0.30,
        contradiction_count: 2,
        revision_action_count: 2,
        memory_feedback_updates: 1,
        route_attention_fraction: 0.84,
        agent_team_collision_free: false,
        drift_memory_write_allowed: false,
        runtime_kv_hold: true,
        ..healthy_input()
    }
}

fn require_at_least(failures: &mut Vec<String>, metric: &str, actual: usize, required: usize) {
    if actual < required {
        failures.push(format!("{metric} {actual} below required {required}"));
    }
}

fn all_finite(values: impl IntoIterator<Item = f32>) -> bool {
    values.into_iter().all(f32::is_finite)
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

fn digest<'a>(parts: impl IntoIterator<Item = &'a str>) -> String {
    stable_redaction_digest(parts)
}

fn slug(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}
