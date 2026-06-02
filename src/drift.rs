use crate::router::{GenerationMetrics, RouteBudget};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriftSeverity {
    Stable,
    Watch,
    Block,
    Rollback,
}

impl DriftSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Watch => "watch",
            Self::Block => "block",
            Self::Rollback => "rollback",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DriftReport {
    pub severity: DriftSeverity,
    pub allow_memory_write: bool,
    pub allow_runtime_kv_write: bool,
    pub penalize_used_memory: bool,
    pub rollback_adaptive: bool,
    pub notes: Vec<String>,
}

impl DriftReport {
    pub fn summary(&self) -> String {
        format!(
            "severity={} memory_write={} runtime_kv_write={} penalize_used_memory={} rollback_adaptive={} notes={}",
            self.severity.as_str(),
            self.allow_memory_write,
            self.allow_runtime_kv_write,
            self.penalize_used_memory,
            self.rollback_adaptive,
            self.notes.len()
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DriftInput {
    pub quality: f32,
    pub contradiction_count: usize,
    pub metrics: GenerationMetrics,
    pub route_budget: RouteBudget,
    pub used_memories: usize,
    pub exported_runtime_kv_blocks: usize,
    pub stream_windows: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct DriftGuard {
    pub min_memory_quality: f32,
    pub min_runtime_kv_quality: f32,
    pub rollback_quality: f32,
    pub max_contradictions: usize,
    pub max_perplexity: f32,
}

impl Default for DriftGuard {
    fn default() -> Self {
        Self {
            min_memory_quality: 0.52,
            min_runtime_kv_quality: 0.60,
            rollback_quality: 0.30,
            max_contradictions: 1,
            max_perplexity: 34.0,
        }
    }
}

impl DriftGuard {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn evaluate(&self, input: DriftInput) -> DriftReport {
        let quality = input.quality.clamp(0.0, 1.0);
        let mut notes = Vec::new();

        if quality <= self.rollback_quality {
            notes.push(format!(
                "quality:{quality:.3}:below_rollback:{:.3}",
                self.rollback_quality
            ));
        }
        if input.metrics.perplexity >= self.max_perplexity {
            notes.push(format!(
                "perplexity:{:.3}:above_max:{:.3}",
                input.metrics.perplexity, self.max_perplexity
            ));
        }
        if input.contradiction_count > self.max_contradictions {
            notes.push(format!(
                "contradictions:{}:above_max:{}",
                input.contradiction_count, self.max_contradictions
            ));
        }

        let rollback = quality <= self.rollback_quality
            || input.metrics.perplexity >= self.max_perplexity
            || input.contradiction_count > self.max_contradictions;
        if rollback {
            return DriftReport {
                severity: DriftSeverity::Rollback,
                allow_memory_write: false,
                allow_runtime_kv_write: false,
                penalize_used_memory: input.used_memories > 0,
                rollback_adaptive: true,
                notes,
            };
        }

        if input.contradiction_count > 0 || quality < self.min_memory_quality {
            if input.contradiction_count > 0 {
                notes.push(format!("contradictions:{}", input.contradiction_count));
            }
            if quality < self.min_memory_quality {
                notes.push(format!(
                    "quality:{quality:.3}:below_memory:{:.3}",
                    self.min_memory_quality
                ));
            }
            return DriftReport {
                severity: DriftSeverity::Block,
                allow_memory_write: false,
                allow_runtime_kv_write: false,
                penalize_used_memory: input.used_memories > 0,
                rollback_adaptive: false,
                notes,
            };
        }

        if quality < self.min_runtime_kv_quality || input.metrics.semantic_consistency < 0.58 {
            notes.push(format!(
                "runtime_kv_held:quality={quality:.3}:semantic={:.3}",
                input.metrics.semantic_consistency
            ));
            return DriftReport {
                severity: DriftSeverity::Watch,
                allow_memory_write: true,
                allow_runtime_kv_write: false,
                penalize_used_memory: false,
                rollback_adaptive: false,
                notes,
            };
        }

        if input.exported_runtime_kv_blocks > 0 {
            notes.push(format!(
                "runtime_kv_candidate:{}",
                input.exported_runtime_kv_blocks
            ));
        }
        if input.route_budget.attention_fraction < 0.10 && quality < 0.72 {
            notes.push("route:fast_path_watch".to_owned());
            return DriftReport {
                severity: DriftSeverity::Watch,
                allow_memory_write: true,
                allow_runtime_kv_write: false,
                penalize_used_memory: false,
                rollback_adaptive: false,
                notes,
            };
        }
        if input.stream_windows > 48 {
            notes.push(format!("stream:many_windows={}", input.stream_windows));
        }

        DriftReport {
            severity: if notes.is_empty() {
                DriftSeverity::Stable
            } else {
                DriftSeverity::Watch
            },
            allow_memory_write: true,
            allow_runtime_kv_write: true,
            penalize_used_memory: false,
            rollback_adaptive: false,
            notes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_high_quality_output_can_write_memory_and_runtime_kv() {
        let report = DriftGuard::new().evaluate(input(0.86, 0, 8.0, 0.72, 1));

        assert_eq!(report.severity, DriftSeverity::Watch);
        assert!(report.allow_memory_write);
        assert!(report.allow_runtime_kv_write);
        assert!(!report.rollback_adaptive);
    }

    #[test]
    fn contradiction_blocks_memory_without_rollback() {
        let report = DriftGuard::new().evaluate(input(0.72, 1, 9.0, 0.72, 1));

        assert_eq!(report.severity, DriftSeverity::Block);
        assert!(!report.allow_memory_write);
        assert!(!report.allow_runtime_kv_write);
        assert!(!report.rollback_adaptive);
    }

    #[test]
    fn severe_low_quality_rolls_back_adaptive_state() {
        let report = DriftGuard::new().evaluate(input(0.22, 0, 10.0, 0.20, 0));

        assert_eq!(report.severity, DriftSeverity::Rollback);
        assert!(!report.allow_memory_write);
        assert!(report.rollback_adaptive);
    }

    #[test]
    fn fast_path_watch_holds_runtime_kv_but_keeps_memory_write() {
        let mut input = input(0.68, 0, 9.0, 0.70, 1);
        input.route_budget.attention_tokens = 0;
        input.route_budget.fast_tokens = 12;
        input.route_budget.attention_fraction = 0.0;

        let report = DriftGuard::new().evaluate(input);

        assert_eq!(report.severity, DriftSeverity::Watch);
        assert!(report.allow_memory_write);
        assert!(!report.allow_runtime_kv_write);
        assert!(
            report
                .notes
                .iter()
                .any(|note| note == "route:fast_path_watch")
        );
    }

    fn input(
        quality: f32,
        contradiction_count: usize,
        perplexity: f32,
        semantic_consistency: f32,
        exported_runtime_kv_blocks: usize,
    ) -> DriftInput {
        DriftInput {
            quality,
            contradiction_count,
            metrics: GenerationMetrics {
                perplexity,
                semantic_consistency,
                contradiction_count,
                token_count: 32,
            },
            route_budget: RouteBudget {
                threshold: 0.5,
                attention_tokens: 1,
                fast_tokens: 1,
                attention_fraction: 0.5,
            },
            used_memories: 1,
            exported_runtime_kv_blocks,
            stream_windows: 2,
        }
    }
}
