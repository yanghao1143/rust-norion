use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskProfile {
    General,
    Coding,
    Writing,
    LongDocument,
}

impl FromStr for TaskProfile {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "general" | "通用" => Ok(Self::General),
            "coding" | "code" | "rust" | "代码" | "编程" => Ok(Self::Coding),
            "writing" | "write" | "小说" | "写作" => Ok(Self::Writing),
            "long" | "longdoc" | "long-document" | "document" | "长文档" => {
                Ok(Self::LongDocument)
            }
            other => Err(format!("unknown task profile: {other}")),
        }
    }
}
