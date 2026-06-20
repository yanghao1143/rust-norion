use crate::experience::evidence::{ExperienceEvidenceNote, evidence_notes_by_kind};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiveMemoryFeedbackStats {
    pub reinforced: usize,
    pub penalized: usize,
    pub reinforcement_amount: f32,
    pub penalty_amount: f32,
    pub applied: usize,
    pub removed: usize,
    pub missing: usize,
    pub strength_delta: f32,
    pub detailed_evidence: bool,
}

impl LiveMemoryFeedbackStats {
    pub fn from_notes(notes: &[String]) -> Option<Self> {
        Self::from_evidence_notes(evidence_notes_by_kind(notes, "memory_feedback"))
    }

    pub fn from_notes_for_source(notes: &[String], source: &str) -> Option<Self> {
        Self::from_evidence_notes(
            evidence_notes_by_kind(notes, "memory_feedback")
                .filter(|note| note.first_tag_matches(source)),
        )
    }

    fn from_evidence_notes(notes: impl Iterator<Item = ExperienceEvidenceNote>) -> Option<Self> {
        let mut aggregate: Option<Self> = None;
        for note in notes {
            let Some(stats) = Self::from_note(&note) else {
                continue;
            };
            match &mut aggregate {
                Some(total) => total.absorb(stats),
                None => aggregate = Some(stats),
            }
        }

        aggregate
    }

    fn from_note(note: &ExperienceEvidenceNote) -> Option<Self> {
        let applied = note.field_usize("applied");
        let removed = note.field_usize("removed");
        let missing = note.field_usize("missing");
        let strength_delta = note.field_f32("strength_delta");
        let stats = Self {
            reinforced: note.field_usize("reinforced").unwrap_or(0),
            penalized: note.field_usize("penalized").unwrap_or(0),
            reinforcement_amount: note.field_f32("reinforcement_amount").unwrap_or(0.0),
            penalty_amount: note.field_f32("penalty_amount").unwrap_or(0.0),
            applied: applied.unwrap_or(0),
            removed: removed.unwrap_or(0),
            missing: missing.unwrap_or(0),
            strength_delta: strength_delta.unwrap_or(0.0),
            detailed_evidence: applied.is_some()
                && removed.is_some()
                && missing.is_some()
                && strength_delta.is_some(),
        };

        stats.has_updates().then_some(stats)
    }

    fn absorb(&mut self, other: Self) {
        self.reinforced = self.reinforced.saturating_add(other.reinforced);
        self.penalized = self.penalized.saturating_add(other.penalized);
        self.reinforcement_amount += other.reinforcement_amount;
        self.penalty_amount += other.penalty_amount;
        self.applied = self.applied.saturating_add(other.applied);
        self.removed = self.removed.saturating_add(other.removed);
        self.missing = self.missing.saturating_add(other.missing);
        self.strength_delta += other.strength_delta;
        self.detailed_evidence &= other.detailed_evidence;
    }

    pub fn updates(&self) -> usize {
        self.reinforced + self.penalized
    }

    pub fn reinforcement_average(&self) -> Option<f32> {
        (self.reinforced > 0).then(|| self.reinforcement_amount / self.reinforced as f32)
    }

    pub fn penalty_average(&self) -> Option<f32> {
        (self.penalized > 0).then(|| self.penalty_amount / self.penalized as f32)
    }

    pub fn has_detailed_update_evidence(&self) -> bool {
        self.detailed_evidence
            && self.applied.saturating_add(self.missing) == self.updates()
            && self.removed <= self.applied
            && self.strength_delta.is_finite()
            && self.strength_delta >= 0.0
    }

    pub fn applied_ratio(&self) -> Option<f32> {
        (self.has_detailed_update_evidence() && self.updates() > 0)
            .then(|| self.applied as f32 / self.updates() as f32)
    }

    fn has_updates(&self) -> bool {
        self.updates() > 0
            && self.reinforcement_amount.is_finite()
            && self.penalty_amount.is_finite()
    }
}

pub(super) fn nonnegative_f32(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecursiveReplayStats {
    pub chunks: Option<usize>,
    pub merge_rounds: Option<usize>,
    pub waves: Option<usize>,
    pub parallel: Option<usize>,
    pub runtime_calls: Option<usize>,
}

impl RecursiveReplayStats {
    pub fn from_notes(notes: &[String]) -> Option<Self> {
        let mut aggregate: Option<Self> = None;
        for note in evidence_notes_by_kind(notes, "recursive") {
            let stats = Self {
                chunks: note.field_positive_usize("chunks"),
                merge_rounds: note.field_positive_usize("merge_rounds"),
                waves: note.field_positive_usize("waves"),
                parallel: note.field_positive_usize("parallel"),
                runtime_calls: note.field_positive_usize("runtime_calls"),
            };

            if !stats.has_evidence() {
                continue;
            }
            match &mut aggregate {
                Some(total) => total.absorb(stats),
                None => aggregate = Some(stats),
            }
        }

        aggregate
    }

    fn absorb(&mut self, other: Self) {
        self.chunks = max_optional_usize(self.chunks, other.chunks);
        self.merge_rounds = max_optional_usize(self.merge_rounds, other.merge_rounds);
        self.waves = max_optional_usize(self.waves, other.waves);
        self.parallel = max_optional_usize(self.parallel, other.parallel);
        self.runtime_calls = max_optional_usize(self.runtime_calls, other.runtime_calls);
    }

    fn has_evidence(self) -> bool {
        self.chunks.is_some()
            || self.merge_rounds.is_some()
            || self.waves.is_some()
            || self.parallel.is_some()
            || self.runtime_calls.is_some()
    }
}

fn max_optional_usize(left: Option<usize>, right: Option<usize>) -> Option<usize> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RustCheckReplayStats {
    pub passed: usize,
    pub failed: usize,
    pub diagnostic_chars: usize,
}

impl RustCheckReplayStats {
    pub fn from_notes(notes: &[String]) -> Option<Self> {
        let mut stats = Self::default();
        for note in evidence_notes_by_kind(notes, "rust_check") {
            match note.field_bool("passed") {
                Some(true) => stats.passed = stats.passed.saturating_add(1),
                Some(false) => stats.failed = stats.failed.saturating_add(1),
                None => {}
            }
            stats.diagnostic_chars = stats
                .diagnostic_chars
                .saturating_add(note.field_usize("diagnostic_chars").unwrap_or(0));
        }

        stats.has_evidence().then_some(stats)
    }

    pub fn items(self) -> usize {
        self.passed.saturating_add(self.failed)
    }

    fn has_evidence(self) -> bool {
        self.items() > 0 || self.diagnostic_chars > 0
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BusinessContractReplayStats {
    pub passed: usize,
    pub failed: usize,
    pub raw_passed: usize,
    pub raw_failed: usize,
    pub response_normalized: usize,
    pub sanitized: usize,
    pub canonical_fallbacks: usize,
}

impl BusinessContractReplayStats {
    pub fn from_notes(notes: &[String]) -> Option<Self> {
        let mut stats = Self::default();
        for note in evidence_notes_by_kind(notes, "business_contract") {
            match note.field_bool("passed") {
                Some(true) => stats.passed = stats.passed.saturating_add(1),
                Some(false) => stats.failed = stats.failed.saturating_add(1),
                None => {}
            }
            match note.field_bool("raw_passed") {
                Some(true) => stats.raw_passed = stats.raw_passed.saturating_add(1),
                Some(false) => stats.raw_failed = stats.raw_failed.saturating_add(1),
                None => {}
            }
            stats.response_normalized = stats.response_normalized.saturating_add(usize::from(
                note.field_bool("response_normalized").unwrap_or(false),
            ));
            stats.sanitized = stats.sanitized.saturating_add(usize::from(
                note.field_matches("normalization", "sanitized"),
            ));
            stats.canonical_fallbacks = stats.canonical_fallbacks.saturating_add(usize::from(
                note.field_bool("canonical_fallback").unwrap_or(false),
            ));
        }

        stats.has_evidence().then_some(stats)
    }

    pub fn items(self) -> usize {
        self.passed.saturating_add(self.failed)
    }

    fn has_evidence(self) -> bool {
        self.items() > 0
            || self.raw_passed > 0
            || self.raw_failed > 0
            || self.response_normalized > 0
            || self.sanitized > 0
            || self.canonical_fallbacks > 0
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PoolDispatchReplayStats {
    pub items: usize,
    pub forwarded: usize,
    pub clamped: usize,
    pub low_priority: usize,
    pub selected_roles: Vec<String>,
}

impl PoolDispatchReplayStats {
    pub fn from_notes(notes: &[String]) -> Option<Self> {
        let mut stats = Self::default();
        for note in evidence_notes_by_kind(notes, "pool_dispatch") {
            stats.items = stats.items.saturating_add(1);
            stats.forwarded = stats
                .forwarded
                .saturating_add(usize::from(note.field_bool("forwarded").unwrap_or(false)));
            stats.clamped = stats.clamped.saturating_add(usize::from(
                note.field_bool("max_tokens_clamped").unwrap_or(false),
            ));
            stats.low_priority = stats.low_priority.saturating_add(usize::from(
                note.field_bool("low_priority").unwrap_or(false),
            ));
            if let Some(role) = note
                .field_normalized_ascii_trimmed("selected_role")
                .map(|role| role.to_ascii_lowercase())
                && !stats.selected_roles.iter().any(|item| item == &role)
            {
                stats.selected_roles.push(role);
            }
        }

        (stats.items > 0).then_some(stats)
    }
}

pub(super) fn recursive_call_pressure(
    recursive_runtime_calls: Option<usize>,
    recursive_stats: Option<RecursiveReplayStats>,
    token_count: usize,
) -> f32 {
    let Some(calls) = recursive_runtime_calls else {
        return 0.0;
    };

    let expected_calls = recursive_stats
        .and_then(|stats| stats.chunks)
        .unwrap_or_else(|| token_count.max(1))
        .max(1);
    if calls <= expected_calls {
        return 0.0;
    }

    let excess_pressure =
        calls.saturating_sub(expected_calls) as f32 / (expected_calls.max(4) * 3) as f32;
    let wave_pressure = recursive_stats
        .and_then(|stats| stats.waves)
        .map(|waves| (waves.saturating_sub(1) as f32 / 48.0).min(0.10))
        .unwrap_or(0.0);
    let parallel_relief = recursive_stats
        .and_then(|stats| stats.parallel)
        .map(|parallel| ((parallel.saturating_sub(1) as f32) * 0.015).min(0.05))
        .unwrap_or(0.0);

    (excess_pressure + wave_pressure - parallel_relief).clamp(0.0, 0.35)
}
