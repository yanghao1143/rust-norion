use rust_norion::TaskProfile;

pub(crate) fn detect_profile(prompt: &str) -> TaskProfile {
    let lower = prompt.to_ascii_lowercase();

    if contains_any(
        &lower,
        &[
            "rust",
            "cargo",
            "crate",
            "code",
            "api",
            "struct",
            "trait",
            "impl",
            "borrow",
            "ownership",
            "lifetime",
            "tokio",
            "axum",
            "clippy",
        ],
    ) || contains_any(
        prompt,
        &[
            "代码",
            "编译",
            "函数",
            "结构体",
            "特征",
            "接口",
            "所有权",
            "借用",
            "生命周期",
        ],
    ) {
        TaskProfile::Coding
    } else if lower.contains("novel") || lower.contains("story") || lower.contains("writing") {
        TaskProfile::Writing
    } else if lower.contains("document")
        || lower.contains("context")
        || lower.contains("million token")
    {
        TaskProfile::LongDocument
    } else {
        TaskProfile::General
    }
}

fn contains_any(text: &str, markers: &[&str]) -> bool {
    markers.iter().any(|marker| text.contains(marker))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_chinese_coding_prompts_without_explicit_profile() {
        assert_eq!(
            detect_profile("请审查这段代码并指出编译问题"),
            TaskProfile::Coding
        );
        assert_eq!(
            detect_profile("帮我写一个函数处理配置"),
            TaskProfile::Coding
        );
        assert_eq!(
            detect_profile("解释这个接口和结构体的实现关系"),
            TaskProfile::Coding
        );
        assert_eq!(
            detect_profile("解释所有权和生命周期规则"),
            TaskProfile::Coding
        );
        assert_eq!(
            detect_profile("Explain ownership and lifetime rules"),
            TaskProfile::Coding
        );
    }
}
