use super::output::DEFAULT_EDITION;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustSnippetCheck {
    pub code: String,
    pub edition: String,
    pub case_name: Option<String>,
}

impl RustSnippetCheck {
    pub fn new(code: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            edition: DEFAULT_EDITION.to_owned(),
            case_name: None,
        }
    }

    pub fn with_edition(mut self, edition: impl Into<String>) -> Self {
        self.edition = edition.into();
        self
    }

    pub fn with_case_name(mut self, case_name: impl Into<String>) -> Self {
        self.case_name = Some(case_name.into());
        self
    }
}
