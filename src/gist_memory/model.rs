#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GistLevel {
    Document,
    Section,
    Paragraph,
}

impl GistLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::Section => "section",
            Self::Paragraph => "paragraph",
        }
    }
}

impl std::str::FromStr for GistLevel {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "document" => Ok(Self::Document),
            "section" => Ok(Self::Section),
            "paragraph" => Ok(Self::Paragraph),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GistRecord {
    pub level: GistLevel,
    pub title: String,
    pub summary: String,
    pub source_tokens: usize,
    pub importance: f32,
}

impl GistRecord {
    pub fn hint(&self) -> String {
        format!(
            "{}:{} importance={:.3} tokens={} summary={}",
            self.level.as_str(),
            self.title,
            self.importance,
            self.source_tokens,
            self.summary
        )
    }
}
