use crate::hierarchy::TaskProfile;

pub(super) fn profile_to_str(profile: TaskProfile) -> &'static str {
    match profile {
        TaskProfile::General => "general",
        TaskProfile::Coding => "coding",
        TaskProfile::Writing => "writing",
        TaskProfile::LongDocument => "long_document",
    }
}

pub(super) fn str_to_profile(value: &str) -> Option<TaskProfile> {
    match value {
        "general" => Some(TaskProfile::General),
        "coding" => Some(TaskProfile::Coding),
        "writing" => Some(TaskProfile::Writing),
        "long_document" => Some(TaskProfile::LongDocument),
        _ => None,
    }
}
