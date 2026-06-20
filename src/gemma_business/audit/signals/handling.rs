use super::business_answer_contains_signal;

pub(super) fn business_answer_contains_handling_signal_impl(answer: &str, lower: &str) -> bool {
    const ASCII_SIGNALS: &[&str] = &[
        "business",
        "routing",
        "route",
        "feedback",
        "memory",
        "rust",
        "handled",
        "supports",
        "confirmed",
        "confirm",
        "request",
        "runtimebackend",
        "noiron",
        "gemma",
        "local",
    ];
    const TEXT_SIGNALS: &[&str] = &[
        "业务", "路由", "反馈", "记忆", "内存", "处理", "支持", "确认", "请求", "本地",
    ];
    ASCII_SIGNALS
        .iter()
        .any(|phrase| business_answer_contains_signal(answer, lower, phrase))
        || TEXT_SIGNALS
            .iter()
            .any(|phrase| business_answer_contains_signal(answer, lower, phrase))
}
