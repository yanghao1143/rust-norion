use crate::router::RouteBudget;

pub(super) fn serialize_route_budget(route_budget: RouteBudget) -> String {
    format!(
        "{:.6},{},{},{:.6}",
        route_budget.threshold,
        route_budget.attention_tokens,
        route_budget.fast_tokens,
        route_budget.attention_fraction
    )
}

pub(super) fn deserialize_route_budget(value: &str) -> Option<RouteBudget> {
    let fields = value.split(',').collect::<Vec<_>>();
    if fields.len() != 4 {
        return None;
    }

    Some(RouteBudget {
        threshold: fields[0].parse::<f32>().ok()?,
        attention_tokens: fields[1].parse::<usize>().ok()?,
        fast_tokens: fields[2].parse::<usize>().ok()?,
        attention_fraction: fields[3].parse::<f32>().ok()?.clamp(0.0, 1.0),
    })
}
