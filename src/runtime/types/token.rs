#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeTokenId {
    pub id: u32,
    pub text: String,
}

impl RuntimeTokenId {
    pub fn new(id: u32, text: impl Into<String>) -> Self {
        Self {
            id,
            text: text.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeToken {
    pub text: String,
    pub logprob: Option<f32>,
    pub entropy: Option<f32>,
}

impl RuntimeToken {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            logprob: None,
            entropy: None,
        }
    }
}
