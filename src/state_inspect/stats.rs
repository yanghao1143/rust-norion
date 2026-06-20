use crate::experience::evidence::evidence_notes_by_kind;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RustCheckInspectionStats {
    pub passed: usize,
    pub failed: usize,
    pub diagnostic_chars: usize,
}

impl RustCheckInspectionStats {
    pub(super) fn from_notes(notes: &[String]) -> Option<Self> {
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

    pub fn total(self) -> usize {
        self.passed.saturating_add(self.failed)
    }

    fn has_evidence(self) -> bool {
        self.total() > 0 || self.diagnostic_chars > 0
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RuntimeErrorInspectionStats {
    pub errors: usize,
    pub timeouts: usize,
    pub message_chars: usize,
}

impl RuntimeErrorInspectionStats {
    pub(super) fn from_notes(notes: &[String]) -> Option<Self> {
        let mut stats = Self::default();
        for note in evidence_notes_by_kind(notes, "runtime_error") {
            stats.errors = stats.errors.saturating_add(1);
            if note.field_bool("timeout").unwrap_or(false) {
                stats.timeouts = stats.timeouts.saturating_add(1);
            }
            stats.message_chars = stats
                .message_chars
                .saturating_add(note.field_usize("message_chars").unwrap_or(0));
        }

        (stats.errors > 0).then_some(stats)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BusinessContractInspectionStats {
    pub passed: usize,
    pub failed: usize,
    pub required_signals: usize,
    pub matched_signals: usize,
    pub missing_signals: usize,
    pub protocol_leaks: usize,
    pub substitutions: usize,
    pub evasive_denials: usize,
    pub missing_handling_signals: usize,
    pub raw_passed: usize,
    pub raw_failed: usize,
    pub response_normalized: usize,
    pub sanitized: usize,
    pub canonical_fallbacks: usize,
}

impl BusinessContractInspectionStats {
    pub(super) fn from_notes(notes: &[String]) -> Option<Self> {
        let mut stats = Self::default();
        for note in evidence_notes_by_kind(notes, "business_contract") {
            match note.field_bool("passed") {
                Some(true) => stats.passed = stats.passed.saturating_add(1),
                Some(false) => stats.failed = stats.failed.saturating_add(1),
                None => {}
            }
            stats.required_signals = stats
                .required_signals
                .saturating_add(note.field_usize("required").unwrap_or(0));
            stats.matched_signals = stats
                .matched_signals
                .saturating_add(note.field_usize("matched").unwrap_or(0));
            stats.missing_signals = stats
                .missing_signals
                .saturating_add(note.field_usize("missing").unwrap_or(0));
            stats.protocol_leaks = stats.protocol_leaks.saturating_add(usize::from(
                note.field_bool("protocol_leak").unwrap_or(false),
            ));
            stats.substitutions = stats.substitutions.saturating_add(usize::from(
                note.field_bool("substituted_runtime_model_experiences")
                    .unwrap_or(false),
            ));
            stats.evasive_denials = stats.evasive_denials.saturating_add(usize::from(
                note.field_bool("evasive_denial").unwrap_or(false),
            ));
            stats.missing_handling_signals = stats.missing_handling_signals.saturating_add(
                usize::from(note.field_bool("handling_signal") == Some(false)),
            );
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

    pub fn total(self) -> usize {
        self.passed.saturating_add(self.failed)
    }

    fn has_evidence(self) -> bool {
        self.total() > 0
            || self.required_signals > 0
            || self.matched_signals > 0
            || self.missing_signals > 0
            || self.protocol_leaks > 0
            || self.substitutions > 0
            || self.evasive_denials > 0
            || self.missing_handling_signals > 0
            || self.raw_passed > 0
            || self.raw_failed > 0
            || self.response_normalized > 0
            || self.sanitized > 0
            || self.canonical_fallbacks > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_check_stats_ignore_missing_or_malformed_bool_outcomes() {
        let stats = RustCheckInspectionStats::from_notes(&[
            "rust_check:label=legacy_rustc:diagnostic_chars=11".to_owned(),
            "rust_check:passed=maybe:label=broken_rustc:diagnostic_chars=7".to_owned(),
            "rust_check:passed=false:label=rustc_failed:diagnostic_chars=13".to_owned(),
        ])
        .unwrap();

        assert_eq!(
            stats,
            RustCheckInspectionStats {
                passed: 0,
                failed: 1,
                diagnostic_chars: 31,
            }
        );
    }

    #[test]
    fn rust_check_stats_keep_diagnostics_without_outcome() {
        let stats = RustCheckInspectionStats::from_notes(&[
            "rust_check:label=legacy_rustc:diagnostic_chars=11".to_owned(),
            "rust_check:passed=maybe:label=broken_rustc:diagnostic_chars=7".to_owned(),
        ])
        .unwrap();

        assert_eq!(
            stats,
            RustCheckInspectionStats {
                passed: 0,
                failed: 0,
                diagnostic_chars: 18,
            }
        );
    }

    #[test]
    fn runtime_error_stats_ignore_unknown_timeout_without_hiding_error_evidence() {
        let stats = RuntimeErrorInspectionStats::from_notes(&[
            "runtime_error:label=slow-first-token:timeout=maybe:message_chars=17".to_owned(),
            "runtime_error:label=slow-first-token:timeout=false:message_chars=19".to_owned(),
            "ｒｕｎｔｉｍｅ＿ｅｒｒｏｒ：ｌａｂｅｌ＝deadline：ｔｉｍｅｏｕｔ＝ TRUE ：ｍｅｓｓａｇｅ＿ｃｈａｒｓ＝２３"
                .to_owned(),
        ])
        .unwrap();

        assert_eq!(
            stats,
            RuntimeErrorInspectionStats {
                errors: 3,
                timeouts: 1,
                message_chars: 59,
            }
        );
    }

    #[test]
    fn business_contract_stats_ignore_unknown_bool_outcomes_without_hiding_other_evidence() {
        let stats = BusinessContractInspectionStats::from_notes(&[
            "business_contract:case=legacy-audit:required=4:matched=3:missing=1:normalization=sanitized"
                .to_owned(),
            "business_contract:case=broken-audit:passed=maybe:raw_passed=:handling_signal=:response_normalized=true"
                .to_owned(),
            "business_contract:case=explicit-audit:passed=false:raw_passed=true:handling_signal=false"
                .to_owned(),
        ])
        .unwrap();

        assert_eq!(
            stats,
            BusinessContractInspectionStats {
                passed: 0,
                failed: 1,
                required_signals: 4,
                matched_signals: 3,
                missing_signals: 1,
                protocol_leaks: 0,
                substitutions: 0,
                evasive_denials: 0,
                missing_handling_signals: 1,
                raw_passed: 1,
                raw_failed: 0,
                response_normalized: 1,
                sanitized: 1,
                canonical_fallbacks: 0,
            }
        );
    }

    #[test]
    fn business_contract_stats_count_only_explicit_bool_risk_flags() {
        let stats = BusinessContractInspectionStats::from_notes(&[
            "business_contract:case=risk:protocol_leak=TRUE:substituted_runtime_model_experiences=true:evasive_denial=true:handling_signal=false:response_normalized=true:canonical_fallback=true"
                .to_owned(),
            "business_contract:case=clean:protocol_leak=false:substituted_runtime_model_experiences=false:evasive_denial=false:handling_signal=true:response_normalized=false:canonical_fallback=false"
                .to_owned(),
            "business_contract:case=malformed:protocol_leak=maybe:substituted_runtime_model_experiences=:evasive_denial=1:handling_signal=:response_normalized=yes:canonical_fallback=unknown"
                .to_owned(),
            "ｂｕｓｉｎｅｓｓ＿ｃｏｎｔｒａｃｔ：ｃａｓｅ＝full-width：ｐｒｏｔｏｃｏｌ＿ｌｅａｋ＝ｔｒｕｅ：ｓｕｂｓｔｉｔｕｔｅｄ＿ｒｕｎｔｉｍｅ＿ｍｏｄｅｌ＿ｅｘｐｅｒｉｅｎｃｅｓ＝ｔｒｕｅ：ｅｖａｓｉｖｅ＿ｄｅｎｉａｌ＝ｔｒｕｅ：ｈａｎｄｌｉｎｇ＿ｓｉｇｎａｌ＝ｆａｌｓｅ：ｒｅｓｐｏｎｓｅ＿ｎｏｒｍａｌｉｚｅｄ＝ｔｒｕｅ：ｃａｎｏｎｉｃａｌ＿ｆａｌｌｂａｃｋ＝ｔｒｕｅ"
                .to_owned(),
        ])
        .unwrap();

        assert_eq!(stats.protocol_leaks, 2);
        assert_eq!(stats.substitutions, 2);
        assert_eq!(stats.evasive_denials, 2);
        assert_eq!(stats.missing_handling_signals, 2);
        assert_eq!(stats.response_normalized, 2);
        assert_eq!(stats.canonical_fallbacks, 2);
        assert_eq!(stats.total(), 0);
    }

    #[test]
    fn business_contract_stats_keep_audit_fields_without_outcome() {
        let stats = BusinessContractInspectionStats::from_notes(&[
            "business_contract:case=legacy-audit:required=4:matched=3:missing=1:normalization=sanitized"
                .to_owned(),
            "business_contract:case=broken-audit:passed=maybe:raw_passed=:handling_signal=:response_normalized=true"
                .to_owned(),
        ])
        .unwrap();

        assert_eq!(
            stats,
            BusinessContractInspectionStats {
                passed: 0,
                failed: 0,
                required_signals: 4,
                matched_signals: 3,
                missing_signals: 1,
                protocol_leaks: 0,
                substitutions: 0,
                evasive_denials: 0,
                missing_handling_signals: 0,
                raw_passed: 0,
                raw_failed: 0,
                response_normalized: 1,
                sanitized: 1,
                canonical_fallbacks: 0,
            }
        );
    }
}
