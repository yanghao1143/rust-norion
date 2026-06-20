use std::collections::HashSet;

use crate::hierarchy::TaskProfile;

use super::types::{Route, RoutingContext};

pub(super) fn tokenize(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();

    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || (!ch.is_ascii() && !ch.is_whitespace()) {
            current.push(ch);
        } else if !current.is_empty() {
            out.push(std::mem::take(&mut current));
        }
    }

    if !current.is_empty() {
        out.push(current);
    }

    out
}

pub(super) fn estimate_token_entropy(token: &str) -> f32 {
    if token.is_empty() {
        return 0.0;
    }

    let len = token.chars().count() as f32;
    let unique = token.chars().collect::<HashSet<_>>().len() as f32;
    let unique_ratio = unique / len.max(1.0);
    let symbol_ratio = token
        .chars()
        .filter(|ch| !ch.is_alphanumeric() && *ch != '_')
        .count() as f32
        / len.max(1.0);
    let digit_ratio = token.chars().filter(|ch| ch.is_ascii_digit()).count() as f32 / len.max(1.0);
    let case_mix = if token.chars().any(|ch| ch.is_ascii_uppercase())
        && token.chars().any(|ch| ch.is_ascii_lowercase())
    {
        0.08
    } else {
        0.0
    };
    let length_pressure = (len / 24.0).min(0.22);

    (unique_ratio * 0.52 + symbol_ratio * 0.16 + digit_ratio * 0.12 + case_mix + length_pressure)
        .clamp(0.0, 1.0)
}

pub(super) fn routing_score(entropy: f32, context: RoutingContext) -> f32 {
    let task_pressure = match context.profile {
        TaskProfile::General => 0.0,
        TaskProfile::Coding => 0.05,
        TaskProfile::Writing => 0.08,
        TaskProfile::LongDocument => 0.10,
    };
    let context_pressure = (context.context_tokens as f32 / 32_000.0).min(0.18);
    let cache_discount = context.cache_hit_rate.clamp(0.0, 1.0) * 0.10;
    let latency_discount = match context.latency_budget_ms {
        Some(budget) if budget <= 150 => 0.10,
        Some(budget) if budget <= 500 => 0.04,
        _ => 0.0,
    };
    let compute_headroom = context.compute_headroom.clamp(0.0, 1.0);
    let hardware_pressure_discount = context.hardware_pressure.clamp(0.0, 1.0) * 0.16;
    let constrained_device_discount = (0.5 - compute_headroom).max(0.0) * 0.10;
    let accelerator_bonus = (compute_headroom - 0.5).max(0.0) * 0.12;

    (entropy * 0.72 + task_pressure + context_pressure + accelerator_bonus
        - cache_discount
        - latency_discount
        - hardware_pressure_discount
        - constrained_device_discount)
        .clamp(0.0, 1.0)
}

pub(super) fn choose_route(score: f32, threshold: f32, context: RoutingContext) -> Route {
    match context.profile {
        TaskProfile::LongDocument if context.context_tokens >= 8_192 => Route::ConvolutionalFusion,
        TaskProfile::LongDocument if score < threshold + 0.18 => Route::ConvolutionalFusion,
        TaskProfile::Coding if score < threshold + 0.24 => Route::LocalWindowAttention,
        TaskProfile::Writing => Route::GlobalAttention,
        _ if score >= threshold + 0.24 => Route::GlobalAttention,
        _ => Route::LocalWindowAttention,
    }
}
