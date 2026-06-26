#[derive(Debug, Clone, PartialEq, Default)]
pub struct RuntimeDiagnostics {
    pub model_id: Option<String>,
    pub selected_adapter: Option<String>,
    pub adapter_cache_mode: Option<String>,
    pub adapter_stream_trace_id: Option<String>,
    pub adapter_stream_gate_summary_digest: Option<String>,
    pub device_profile: Option<String>,
    pub primary_lane: Option<String>,
    pub fallback_lane: Option<String>,
    pub memory_mode: Option<String>,
    pub device_execution_source: Option<String>,
    pub layer_count: usize,
    pub global_layers: usize,
    pub local_window_layers: usize,
    pub convolutional_fusion_layers: usize,
    pub hidden_size: usize,
    pub local_window_tokens: usize,
    pub forward_energy: Option<f32>,
    pub kv_influence: Option<f32>,
    pub imported_kv_blocks: usize,
    pub weak_runtime_kv_imports_skipped: usize,
    pub budget_limited_runtime_kv_imports_skipped: usize,
    pub exported_kv_blocks: usize,
    pub runtime_kv_segments_included: usize,
    pub runtime_kv_segments_skipped: usize,
    pub runtime_kv_segments_rejected: usize,
    pub hot_kv_precision_bits: Option<u8>,
    pub cold_kv_precision_bits: Option<u8>,
}

impl RuntimeDiagnostics {
    pub fn runtime_reported_device_execution_source() -> &'static str {
        "runtime-reported"
    }

    pub fn control_plane_filled_device_execution_source() -> &'static str {
        "control-plane-filled"
    }

    pub fn normalize_device_execution_source(value: impl AsRef<str>) -> Option<String> {
        let value = value.as_ref().trim();
        matches!(value, "runtime-reported" | "control-plane-filled").then(|| value.to_owned())
    }

    pub fn normalize_adapter_cache_mode(value: impl AsRef<str>) -> Option<String> {
        let value = value.as_ref().trim();
        matches!(value, "no_cache" | "chunked_cache" | "genome_filtered").then(|| value.to_owned())
    }

    pub fn normalize_adapter_stream_trace_id(value: impl AsRef<str>) -> Option<String> {
        let value = value.as_ref().trim();
        if value.is_empty()
            || value.len() > 96
            || !value
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, ':' | '-' | '_' | '.'))
        {
            return None;
        }
        Some(value.to_owned())
    }

    pub fn normalize_adapter_stream_gate_summary_digest(value: impl AsRef<str>) -> Option<String> {
        let value = value.as_ref().trim();
        let digest = value.strip_prefix("fnv64:")?;
        if digest.len() == 16 && digest.chars().all(|ch| ch.is_ascii_hexdigit()) {
            Some(value.to_owned())
        } else {
            None
        }
    }

    pub fn empty() -> Self {
        Self::default()
    }

    pub fn has_forward_signal(&self) -> bool {
        self.layer_count > 0
            || self.has_layer_mode_signal()
            || self.has_device_execution_signal()
            || self.forward_energy.is_some()
            || self.kv_influence.is_some()
            || self.has_runtime_kv_activity_signal()
    }

    pub fn has_runtime_architecture_signal(&self) -> bool {
        has_text(self.model_id.as_deref())
            && self.layer_count > 0
            && self.hidden_size > 0
            && self.local_window_tokens > 0
    }

    pub fn layer_mode_count(&self) -> usize {
        self.global_layers
            .saturating_add(self.local_window_layers)
            .saturating_add(self.convolutional_fusion_layers)
    }

    pub fn has_layer_mode_signal(&self) -> bool {
        self.layer_mode_count() > 0
    }

    pub fn has_all_layer_modes(&self) -> bool {
        self.global_layers > 0
            && self.local_window_layers > 0
            && self.convolutional_fusion_layers > 0
    }

    pub fn has_device_profile_signal(&self) -> bool {
        has_text(self.device_profile.as_deref())
    }

    pub fn has_device_execution_signal(&self) -> bool {
        self.has_device_profile_signal()
            && has_text(self.primary_lane.as_deref())
            && has_text(self.fallback_lane.as_deref())
            && has_text(self.memory_mode.as_deref())
    }

    pub fn has_runtime_reported_device_execution_signal(&self) -> bool {
        self.has_device_execution_signal()
            && self.device_execution_source.as_deref()
                == Some(Self::runtime_reported_device_execution_source())
    }

    pub fn has_adapter_stream_trace_signal(&self) -> bool {
        self.adapter_cache_mode.is_some() && has_text(self.adapter_stream_trace_id.as_deref())
    }

    pub fn has_adapter_stream_gate_summary_signal(&self) -> bool {
        self.adapter_cache_mode.is_some()
            && self
                .adapter_stream_gate_summary_digest
                .as_deref()
                .is_some_and(|value| {
                    Self::normalize_adapter_stream_gate_summary_digest(value).is_some()
                })
    }

    pub fn has_control_plane_filled_device_execution_signal(&self) -> bool {
        self.has_device_execution_signal()
            && self.device_execution_source.as_deref()
                == Some(Self::control_plane_filled_device_execution_source())
    }

    pub fn has_valid_kv_precision_signal(&self) -> bool {
        match (self.hot_kv_precision_bits, self.cold_kv_precision_bits) {
            (Some(hot), Some(cold)) => matches!(hot, 4 | 8) && matches!(cold, 4 | 8) && cold <= hot,
            _ => false,
        }
    }

    pub fn runtime_kv_segment_count(&self) -> usize {
        self.runtime_kv_segments_included
            .saturating_add(self.runtime_kv_segments_skipped)
            .saturating_add(self.runtime_kv_segments_rejected)
    }

    pub fn runtime_kv_segment_yield(&self) -> Option<f32> {
        let total = self.runtime_kv_segment_count();
        if total == 0 {
            return None;
        }

        let total = total as f32;
        let included = self.runtime_kv_segments_included as f32 / total;
        let skipped = self.runtime_kv_segments_skipped as f32 / total;
        let rejected = self.runtime_kv_segments_rejected as f32 / total;
        Some((included - skipped * 0.25 - rejected * 0.75).clamp(0.0, 1.0))
    }

    pub fn has_runtime_kv_segment_signal(&self) -> bool {
        self.runtime_kv_segment_count() > 0
    }

    pub fn runtime_kv_activity_count(&self) -> usize {
        self.imported_kv_blocks
            .saturating_add(self.exported_kv_blocks)
            .saturating_add(self.weak_runtime_kv_imports_skipped)
            .saturating_add(self.budget_limited_runtime_kv_imports_skipped)
            .saturating_add(self.runtime_kv_segment_count())
    }

    pub fn has_runtime_kv_exchange_signal(&self) -> bool {
        self.imported_kv_blocks > 0 || self.exported_kv_blocks > 0
    }

    pub fn has_runtime_kv_activity_signal(&self) -> bool {
        self.runtime_kv_activity_count() > 0
    }

    pub fn with_layer_modes(
        mut self,
        global: usize,
        local_window: usize,
        convolutional_fusion: usize,
    ) -> Self {
        self.global_layers = global;
        self.local_window_layers = local_window;
        self.convolutional_fusion_layers = convolutional_fusion;
        self
    }

    pub fn with_device_execution(
        mut self,
        device_profile: impl Into<String>,
        primary_lane: impl Into<String>,
        fallback_lane: impl Into<String>,
        memory_mode: impl Into<String>,
    ) -> Self {
        self.device_profile = non_empty_string(device_profile.into());
        self.primary_lane = non_empty_string(primary_lane.into());
        self.fallback_lane = non_empty_string(fallback_lane.into());
        self.memory_mode = non_empty_string(memory_mode.into());
        self.device_execution_source = self
            .has_device_execution_signal()
            .then(|| Self::runtime_reported_device_execution_source().to_owned());
        self
    }

    pub fn clear_device_execution(mut self) -> Self {
        self.device_profile = None;
        self.primary_lane = None;
        self.fallback_lane = None;
        self.memory_mode = None;
        self.device_execution_source = None;
        self
    }

    pub fn clear_kv_precision(mut self) -> Self {
        self.hot_kv_precision_bits = None;
        self.cold_kv_precision_bits = None;
        self
    }

    pub fn with_kv_precision(mut self, hot_bits: u8, cold_bits: u8) -> Self {
        if matches!(hot_bits, 4 | 8) && matches!(cold_bits, 4 | 8) && cold_bits <= hot_bits {
            self.hot_kv_precision_bits = Some(hot_bits);
            self.cold_kv_precision_bits = Some(cold_bits);
        }
        self
    }
}

fn has_text(value: Option<&str>) -> bool {
    value.is_some_and(|value| !value.trim().is_empty())
}

fn non_empty_string(value: String) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}
