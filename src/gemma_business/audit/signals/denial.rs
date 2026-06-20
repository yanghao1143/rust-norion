pub(super) fn business_answer_contains_evasive_denial_impl(answer: &str, lower: &str) -> bool {
    const ASCII_DENIALS: &[&str] = &[
        "cannot confirm",
        "can't confirm",
        "unable to confirm",
        "not able to confirm",
        "cannot verify",
        "can't verify",
        "unable to verify",
        "not part of",
        "does not belong",
        "not a standard",
        "i cannot",
        "i can't",
        "sorry",
    ];
    const TEXT_DENIALS: &[&str] = &[
        "无法确认",
        "不能确认",
        "无法验证",
        "不能验证",
        "不属于",
        "并不属于",
        "不是标准",
        "无法提供",
        "抱歉",
    ];
    ASCII_DENIALS.iter().any(|phrase| lower.contains(phrase))
        || TEXT_DENIALS.iter().any(|phrase| answer.contains(phrase))
}
