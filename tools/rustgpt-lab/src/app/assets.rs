use std::net::TcpStream;

use crate::http::{write_html, write_static};

const INDEX_HTML: &str = include_str!("../../web/index.html");
const APP_CSS: &str = include_str!("../../web/app.css");
const APP_JS: &str = include_str!("../../web/app.js");

pub(super) fn write_index(stream: &mut TcpStream) -> std::io::Result<()> {
    write_html(stream, INDEX_HTML)
}

pub(super) fn write_css(stream: &mut TcpStream) -> std::io::Result<()> {
    write_static(stream, "text/css; charset=utf-8", APP_CSS)
}

pub(super) fn write_js(stream: &mut TcpStream) -> std::io::Result<()> {
    write_static(stream, "application/javascript; charset=utf-8", APP_JS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_references_split_frontend_assets() {
        assert!(INDEX_HTML.contains(r#"<link rel="stylesheet" href="/app.css">"#));
        assert!(INDEX_HTML.contains(r#"<script src="/app.js"></script>"#));
        assert!(!INDEX_HTML.contains("<style>"));
        assert!(!INDEX_HTML.contains("<script>"));
        assert!(APP_CSS.contains(":root"));
        assert!(APP_JS.contains("chatForm"));
        assert!(APP_JS.contains("sawDone"));
        assert!(APP_JS.contains("stream interrupted"));
        assert!(APP_JS.contains("max_tokens"));
        assert!(APP_JS.contains("const DEFAULT_CONTEXT_MESSAGES = 64;"));
        assert!(APP_JS.contains("const MAX_CONTEXT_MESSAGES = 256;"));
        assert!(APP_JS.contains("const GENERATION_MAX_TOKENS = 262144;"));
        assert!(APP_JS.contains("const DEFAULT_MAX_TOKENS = GENERATION_MAX_TOKENS;"));
        assert!(APP_JS.contains("生成预算 max_tokens=${generationMaxTokens()}"));
        assert!(APP_JS.contains("gemmaRuntimeDetail"));
        assert!(APP_JS.contains("n_ctx"));
        assert!(INDEX_HTML.contains("modelPoolLine"));
        assert!(INDEX_HTML.contains("modelPoolAdviceLine"));
        assert!(APP_JS.contains("modelPoolLine"));
        assert!(APP_JS.contains("modelPoolAdviceLine"));
        assert!(APP_JS.contains("/api/model-pool-status"));
        assert!(APP_JS.contains("/api/model-pool-advice"));
        assert!(APP_JS.contains("function formatModelPoolStatus(pool)"));
        assert!(APP_JS.contains("function formatModelPoolAdvice(pool)"));
        assert!(APP_JS.contains("async function fetchModelPoolAdvice()"));
        assert!(APP_JS.contains("function modelPoolFacts(pool)"));
        assert!(APP_JS.contains("extra_quality_12b_detected"));
        assert!(APP_JS.contains("quality_worker_count"));
        assert!(APP_JS.contains("helper_worker_count"));
        assert!(APP_JS.contains("function helperRoleContractDetail(pool)"));
        assert!(APP_JS.contains("missing_helper_roles"));
        assert!(APP_JS.contains("expected_helper_roles"));
        assert!(APP_JS.contains("recommended_launch_order"));
        assert!(APP_JS.contains("启动顺序"));
        assert!(APP_JS.contains("不要多开 12B"));
        assert!(APP_JS.contains("function formatPoolCapacity(capacity)"));
        assert!(APP_JS.contains("capacity.expansion_allowed"));
        assert!(APP_CSS.contains(".poolline"));
        assert!(APP_CSS.contains(".adviceline"));
        assert!(APP_JS.contains("function recentConversationForSend(limit)"));
        assert!(APP_JS.contains("return keep === 0 ? [] : conversation.slice(-keep);"));
        assert!(!APP_JS.contains("conversation.slice(-(contextLimit - 1))"));
        assert!(APP_JS.contains("发送 ${outgoingMessages.length} 条消息"));
        assert!(!APP_JS.contains("携带 ${outgoingMessages.length} 条上下文消息"));
        assert!(INDEX_HTML.contains(r#"value="262144""#));
        assert!(INDEX_HTML.contains(r#"value="64""#));
        assert!(!APP_JS.contains("max_tokens: 128"));
        assert!(!APP_JS.contains("DEFAULT_MAX_TOKENS = 128"));
        assert!(!APP_JS.contains("MODEL_MAX_TOKENS"));
    }

    #[test]
    fn web_lab_enter_send_and_scroll_guards_stay_in_frontend_asset() {
        assert!(APP_JS.contains("promptEl.addEventListener('keydown'"));
        assert!(APP_JS.contains("rustCheckCode.addEventListener('keydown'"));
        assert!(APP_JS.contains("__rustgptLabAppStarted"));
        assert!(APP_JS.contains("function handleComposerKeydown(event)"));
        assert!(APP_JS.contains("function isPlainEnterSubmit(event)"));
        assert!(APP_JS.contains("event.key === 'Enter'"));
        assert!(APP_JS.contains("!event.shiftKey"));
        assert!(APP_JS.contains("event.isComposing"));
        assert!(APP_JS.contains("event.keyCode !== 229"));
        assert!(APP_JS.contains("event.repeat"));
        assert!(APP_JS.contains("if (!send.disabled) form.requestSubmit();"));
        assert!(!APP_JS.contains(".addEventListener('input'"));
        assert!(!APP_JS.contains("form.submit("));
        assert!(APP_JS.contains("let submitPending = false;"));
        assert!(APP_JS.contains("if (submitPending || inFlight) return;"));
        assert!(APP_JS.contains("send.disabled = submitPending || inFlight || !backendAvailable || !modelReady || backendBusy;"));
        assert!(APP_JS.contains("let backendHealthInFlight = null;"));
        assert!(APP_JS.contains("const STREAM_HEALTH_REFRESH_INTERVAL_MS = 1500;"));
        assert!(APP_JS.contains("function experienceHealthReady(health)"));
        assert!(APP_JS.contains("hygiene.clean === false"));
        assert!(APP_JS.contains("positiveNumber(hygiene.quarantine_candidates)"));
        assert!(APP_JS.contains("positiveNumber(hygiene.repairable_index_records)"));
        assert!(APP_JS.contains("index.retrieval_ready === false"));
        assert!(APP_JS.contains("index.risk_level === 'blocked'"));
        assert!(APP_JS.contains("function experienceHealthDetail(hygiene)"));
        assert!(APP_JS.contains("repair index"));
        assert!(APP_JS.contains("索引"));
        assert!(APP_JS.contains("status-gemma-lab.cmd"));
        assert!(APP_JS.contains("status-built-in-lab.cmd"));
        assert!(!APP_JS.contains("status-forge.cmd"));
        assert!(APP_JS.contains("noisy"));
        assert!(APP_JS.contains("dup"));
        assert!(APP_JS.contains("function refreshBackendHealthThrottled()"));
        assert!(APP_JS.contains("if (backendHealthInFlight) return backendHealthInFlight;"));
        assert!(APP_JS.contains("refreshBackendHealthThrottled();"));
        assert!(APP_JS.contains("followOutput.addEventListener('change'"));
        assert!(APP_JS.contains("mainEl.addEventListener('scroll'"));
        assert!(APP_JS.contains("mainEl.addEventListener('wheel'"));
        assert!(APP_JS.contains("function syncAutoScrollFromViewport()"));
        assert!(APP_JS.contains("suppressScrollSync"));
        assert!(APP_JS.contains("function addStreamMeta(text)"));
        assert!(APP_JS.contains("function upsertStreamProgressMeta(event, data)"));
        assert!(APP_JS.contains("messages.insertBefore(node, activeAssistant);"));
        assert!(APP_JS.contains("function markStreamInterrupted(node, reason)"));
        assert!(APP_JS.contains("if (event === 'delta')"));
        assert!(APP_JS.contains("assistant.textContent += data;"));
        assert!(APP_JS.contains("event === 'status' || event === 'heartbeat'"));
        assert!(APP_JS.contains("upsertStreamProgressMeta(event, data);"));
        assert!(APP_JS.contains("event === 'meta' || event === 'stage'"));
        assert!(APP_CSS.contains(".meta.progress"));
        assert!(APP_JS.contains("let sawFinal = false;"));
        assert!(APP_JS.contains("missing done event before EOF"));
        assert!(!APP_JS.contains("let sawCleanBackendEof = false;"));
        assert!(!APP_JS.contains("backend stream ended before done; keeping received events"));
        assert!(APP_JS.contains("function parseSseData(value)"));
        assert!(APP_JS.contains("let sawSseField = false;"));
        assert!(APP_JS.contains("if (!sawSseField) continue;"));
        assert!(APP_JS.contains("event = parseSseData(normalized.slice(6));"));
        assert!(!APP_JS.contains("normalized.slice(6).trim()"));
        assert!(APP_JS.contains("else if (normalized === 'event')"));
        assert!(APP_JS.contains("event = '';"));
        assert!(APP_JS.contains("if (event.length === 0) event = 'message';"));
        assert!(APP_JS.contains("else if (normalized === 'data')"));
        assert!(APP_JS.contains("data.push('');"));
        assert!(APP_JS.contains("buffer.indexOf('\\r\\r')"));
        assert!(
            APP_JS
                .contains("return boundaries.sort((left, right) => left.index - right.index)[0];")
        );
        assert!(!APP_JS.contains("normalized.slice(5).trimStart()"));
        assert!(APP_CSS.contains("overflow: hidden;"));
        assert!(APP_CSS.contains("min-height: 0;"));
        assert!(APP_CSS.contains("overscroll-behavior: contain;"));
        assert!(APP_CSS.contains(".assistant.interrupted"));
    }

    #[test]
    fn web_lab_heartbeat_events_stay_as_progress_metadata() {
        assert!(INDEX_HTML.contains("状态、心跳和后端事件"));
        assert!(APP_JS.contains("event === 'status' || event === 'heartbeat'"));
        assert!(APP_JS.contains("setStatus(data);"));
        assert!(APP_JS.contains("let activeProgressMeta = null;"));
        assert!(APP_JS.contains("function upsertStreamProgressMeta(event, data)"));
        assert!(APP_JS.contains("activeProgressMeta.textContent = text;"));
        assert!(APP_JS.contains("upsertStreamProgressMeta(event, data);"));
        assert!(APP_JS.contains("activeProgressMeta = null;"));
        assert!(count_occurrences(APP_JS, "activeProgressMeta = null;") >= 3);
        assert!(APP_JS.contains("assistant.textContent += data;"));
        assert!(APP_JS.contains("{ role: 'assistant', content: assistant.textContent.trim() }"));
        assert!(!APP_JS.contains("conversation.push(`${event}: ${data}`)"));
        assert!(!APP_JS.contains("assistant.textContent += `${event}: ${data}`"));
    }

    fn count_occurrences(haystack: &str, needle: &str) -> usize {
        haystack.match_indices(needle).count()
    }
}
