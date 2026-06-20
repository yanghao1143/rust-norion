use crate::hierarchy::TaskProfile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformerTemplateKind {
    GeneralBalanced,
    CodingLocal,
    CreativeWritingGlobal,
    LongContextConvolution,
}

impl TransformerTemplateKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::GeneralBalanced => "general_balanced",
            Self::CodingLocal => "coding_local",
            Self::CreativeWritingGlobal => "creative_writing_global",
            Self::LongContextConvolution => "long_context_convolution",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TransformerTemplate {
    pub kind: TransformerTemplateKind,
    pub global_bias: f32,
    pub local_bias: f32,
    pub convolution_bias: f32,
    pub global_window_scale: f32,
    pub local_window_scale: f32,
    pub convolution_window_scale: f32,
}

impl TransformerTemplate {
    pub fn for_profile(profile: TaskProfile) -> Self {
        match profile {
            TaskProfile::General => Self {
                kind: TransformerTemplateKind::GeneralBalanced,
                global_bias: 0.0,
                local_bias: 0.0,
                convolution_bias: 0.0,
                global_window_scale: 8.0,
                local_window_scale: 1.0,
                convolution_window_scale: 0.5,
            },
            TaskProfile::Coding => Self {
                kind: TransformerTemplateKind::CodingLocal,
                global_bias: -0.02,
                local_bias: 0.12,
                convolution_bias: 0.02,
                global_window_scale: 6.0,
                local_window_scale: 0.75,
                convolution_window_scale: 0.5,
            },
            TaskProfile::Writing => Self {
                kind: TransformerTemplateKind::CreativeWritingGlobal,
                global_bias: 0.12,
                local_bias: -0.02,
                convolution_bias: 0.02,
                global_window_scale: 10.0,
                local_window_scale: 1.25,
                convolution_window_scale: 0.6,
            },
            TaskProfile::LongDocument => Self {
                kind: TransformerTemplateKind::LongContextConvolution,
                global_bias: 0.02,
                local_bias: -0.04,
                convolution_bias: 0.16,
                global_window_scale: 12.0,
                local_window_scale: 1.5,
                convolution_window_scale: 0.75,
            },
        }
    }
}
