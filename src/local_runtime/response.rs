use crate::kv_exchange::RuntimeKvBlock;
use crate::runtime::{RuntimeRequest, RuntimeResponse, RuntimeTokenId};
use crate::runtime_manifest::RuntimeManifest;

use answer::build_answer;
use diagnostics::build_diagnostics;
use evidence::{ResponseEvidence, collect_response_evidence};
use token_accounting::response_tokens;
use trace::build_trace;

use super::forward::LocalForwardState;

mod answer;
mod diagnostics;
mod evidence;
mod token_accounting;
mod trace;

pub(super) fn build_runtime_response(
    request: &RuntimeRequest,
    manifest: &RuntimeManifest,
    tokens: &[RuntimeTokenId],
    imported_kv_blocks: &[RuntimeKvBlock],
    exported_kv_blocks: &[RuntimeKvBlock],
    forward: &LocalForwardState,
) -> RuntimeResponse {
    let evidence = collect_response_evidence(request, manifest, forward);
    let answer = build_answer(
        request,
        manifest,
        tokens,
        imported_kv_blocks,
        forward,
        &evidence,
    );
    let diagnostics = build_diagnostics(
        request,
        manifest,
        imported_kv_blocks,
        exported_kv_blocks,
        forward,
        &evidence,
    );
    let mut response = RuntimeResponse::new(answer.clone()).with_diagnostics(diagnostics);
    response.tokens = response_tokens(&answer);
    response.trace = build_trace(
        request,
        manifest,
        tokens,
        imported_kv_blocks,
        exported_kv_blocks,
        forward,
        &evidence,
    );

    response
}
