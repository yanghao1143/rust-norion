use rust_norion::TaskProfile;

#[derive(Debug, Clone, Copy)]
pub struct GemmaModelServiceBusinessCase {
    pub name: &'static str,
    pub profile: TaskProfile,
    pub prompt: &'static str,
    pub contract_line: &'static str,
    pub required_answer_signals: &'static [&'static str],
}
