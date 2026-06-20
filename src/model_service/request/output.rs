use super::super::json::json_string_field;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum ModelServiceOutputMode {
    #[default]
    Enhanced,
    Raw,
}

impl ModelServiceOutputMode {
    pub(crate) fn parse_from_body(body: &str) -> Result<Self, String> {
        let Some(value) =
            json_string_field(body, "output").or_else(|| json_string_field(body, "mode"))
        else {
            return Ok(Self::default());
        };
        value.parse()
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Enhanced => "enhanced",
            Self::Raw => "raw",
        }
    }
}

impl std::str::FromStr for ModelServiceOutputMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "enhanced" | "noiron" | "default" => Ok(Self::Enhanced),
            "raw" | "gemma" | "runtime" => Ok(Self::Raw),
            _ => Err("output must be enhanced|raw".to_owned()),
        }
    }
}
