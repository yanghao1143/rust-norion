use rust_norion::{TaskProfile, TenantScope};

use super::super::json::{json_string_field, json_usize_field};
use super::scope::parse_tenant_scope;

const MAX_INDEX_CONTEXT_CHARS: usize = 1800;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelServiceExperienceRetrievalRequest {
    pub(crate) prompt: String,
    pub(crate) profile: Option<TaskProfile>,
    pub(crate) limit: Option<usize>,
    pub(crate) index_context: Option<String>,
    pub(crate) tenant_scope: Option<TenantScope>,
}

impl ModelServiceExperienceRetrievalRequest {
    pub(crate) fn index_context_chars(&self) -> usize {
        self.index_context
            .as_deref()
            .map(|value| value.chars().count())
            .unwrap_or(0)
    }

    pub(crate) fn index_context_used(&self) -> bool {
        self.index_context_chars() > 0
    }

    pub(crate) fn effective_retrieval_prompt(&self) -> String {
        let Some(index_context) = self.index_context.as_deref() else {
            return self.prompt.clone();
        };
        format!(
            "Use this SmartSteam model_pool_index repository map as retrieval prefilter context. It may be incomplete; prefer direct prompt intent when there is a conflict.\n\n{index_context}\n\nUser retrieval prompt:\n{}",
            self.prompt
        )
    }
}

pub(super) fn parse_experience_retrieval_request(
    body: &str,
) -> Result<ModelServiceExperienceRetrievalRequest, String> {
    let prompt = json_string_field(body, "prompt")
        .filter(|prompt| !prompt.trim().is_empty())
        .ok_or_else(|| "JSON body must include a non-empty prompt string".to_owned())?;
    let profile = json_string_field(body, "profile")
        .map(|value| value.parse::<TaskProfile>())
        .transpose()
        .map_err(|error| error.to_string())?;
    let limit = json_usize_field(body, "limit").map(|limit| limit.max(1));
    let index_context = json_string_field(body, "index_context")
        .filter(|context| !context.trim().is_empty())
        .map(|context| trim_chars(context.trim(), MAX_INDEX_CONTEXT_CHARS));
    let tenant_scope = parse_tenant_scope(body)?;

    Ok(ModelServiceExperienceRetrievalRequest {
        prompt,
        profile,
        limit,
        index_context,
        tenant_scope,
    })
}

fn trim_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_owned();
    }
    let suffix = "\n[model_pool_index retrieval context truncated]";
    let keep_chars = max_chars.saturating_sub(suffix.chars().count());
    let mut out = value.chars().take(keep_chars).collect::<String>();
    out.push_str(suffix);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_retrieval_request() {
        let request = parse_experience_retrieval_request(
            "{\"prompt\":\"帮我用rust输出for循环\",\"profile\":\"coding\",\"limit\":3}",
        )
        .unwrap();

        assert_eq!(request.prompt, "帮我用rust输出for循环");
        assert_eq!(request.profile, Some(TaskProfile::Coding));
        assert_eq!(request.limit, Some(3));
        assert_eq!(request.index_context, None);
        assert_eq!(request.tenant_scope, None);
        assert_eq!(request.effective_retrieval_prompt(), request.prompt);
    }

    #[test]
    fn parses_and_applies_structured_index_context() {
        let request = parse_experience_retrieval_request(
            "{\"prompt\":\"model pool route code\",\"profile\":\"coding\",\"limit\":3,\"index_context\":\"src/model_service handles route planning\"}",
        )
        .unwrap();

        assert!(request.index_context_used());
        assert_eq!(request.index_context_chars(), 40);
        assert_eq!(request.prompt, "model pool route code");
        assert_eq!(request.tenant_scope, None);
        assert!(
            request
                .effective_retrieval_prompt()
                .contains("SmartSteam model_pool_index repository map")
        );
        assert!(
            request
                .effective_retrieval_prompt()
                .contains("src/model_service handles route planning")
        );
        assert!(
            request
                .effective_retrieval_prompt()
                .contains("User retrieval prompt:\nmodel pool route code")
        );
    }

    #[test]
    fn parses_retrieval_tenant_scope() {
        let request = parse_experience_retrieval_request(
            "{\"prompt\":\"scoped retrieval\",\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"retrieval-1\"}",
        )
        .unwrap();

        assert_eq!(
            request.tenant_scope,
            Some(TenantScope::new("tenant-a", "workspace", "retrieval-1"))
        );
    }

    #[test]
    fn rejects_partial_retrieval_tenant_scope() {
        let error = parse_experience_retrieval_request(
            "{\"prompt\":\"scoped retrieval\",\"tenant_id\":\"tenant-a\"}",
        )
        .unwrap_err();

        assert_eq!(
            error,
            "tenant scope requires tenant_id, workspace_id, and session_id"
        );
    }
}
