use super::{DriftInput, DriftReport, DriftSeverity};

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
