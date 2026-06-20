use crate::provider::StreamRequest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustCheckSettings {
    pub code: Option<String>,
    pub edition: String,
    pub case_name: Option<String>,
}

impl RustCheckSettings {
    pub fn set_code(&mut self, code: impl Into<String>) {
        let code = code.into();
        self.code = (!code.trim().is_empty()).then_some(code);
    }

    pub fn clear_code(&mut self) {
        self.code = None;
    }

    pub fn set_edition(&mut self, edition: impl Into<String>) {
        let edition = edition.into();
        if !edition.trim().is_empty() {
            self.edition = edition;
        }
    }

    pub fn set_case_name(&mut self, case_name: Option<String>) {
        self.case_name = case_name.filter(|case_name| !case_name.trim().is_empty());
    }

    pub fn apply_to_request(&self, request: &mut StreamRequest) {
        request.rust_check_code = self.code.clone();
        request.rust_check_edition = self.edition.clone();
        request.rust_check_case = self.case_name.clone();
    }

    pub fn code_chars(&self) -> usize {
        self.code
            .as_deref()
            .map(|code| code.chars().count())
            .unwrap_or(0)
    }

    pub fn summary(&self) -> String {
        let code = match self.code_chars() {
            0 => "rust_check=off".to_owned(),
            chars => format!("rust_check=on({chars} chars)"),
        };
        let case = self.case_name.as_deref().unwrap_or("none");
        format!(
            "{code} rust_check_edition={} rust_check_case={case}",
            self.edition
        )
    }
}

impl Default for RustCheckSettings {
    fn default() -> Self {
        Self {
            code: None,
            edition: "2021".to_owned(),
            case_name: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::provider::{ChatMessage, StreamEndpoint, StreamRequest};

    use super::*;

    #[test]
    fn applies_rust_check_fields_to_business_cycle_request() {
        let mut settings = RustCheckSettings::default();
        settings.set_code("pub fn ok() {}");
        settings.set_edition("2024");
        settings.set_case_name(Some("smoke".to_owned()));

        let mut request = StreamRequest::chat("check", vec![ChatMessage::user("check")]);
        request.endpoint = StreamEndpoint::BusinessCycle;
        settings.apply_to_request(&mut request);

        let body = request.body_json();

        assert!(body.contains("\"rust_check_code\":\"pub fn ok() {}\""));
        assert!(body.contains("\"rust_check_edition\":\"2024\""));
        assert!(body.contains("\"rust_check_case\":\"smoke\""));
    }
}
