import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const watchdog = setTimeout(() => {
  console.error("operator console inspection lifecycle timed out");
  process.exit(1);
}, 5000);

const html = await readFile(
  new URL("../../src/model_service/console.html", import.meta.url),
  "utf8"
);
const section = (from, to) => {
  const sectionStart = html.indexOf(from);
  const sectionEnd = html.indexOf(to, sectionStart);
  assert.ok(sectionStart >= 0 && sectionEnd > sectionStart, `${from} wiring must exist`);
  return html.slice(sectionStart, sectionEnd);
};
assert.equal(html.includes("setInterval("), false, "inspection must remain event-driven");
assert.match(html.slice(-300), /refreshRuntime\(\);/, "startup must load runtime and DNA state");
assert.match(
  section("async function refreshRuntime", "function extractHtmlArtifact"),
  /renderInspector\(\);[\s\S]*if \(loadInspection && \(state\.activeTab === "dna" \|\| state\.activeTab === "memory"\) && !state\.lastResponse && !state\.busy\) \{[\s\S]*refreshInspection\(\)/,
  "runtime startup must connect DNA and Memory views to persisted inspection"
);
assert.match(
  section('document.querySelectorAll(".tab")', 'document.getElementById("composer")'),
  /reenteringInspection[\s\S]*tab\.dataset\.tab === "memory"[\s\S]*refreshInspection\(true\)/,
  "DNA and Memory re-entry must force a persisted-state refresh"
);
assert.match(
  section('document.getElementById("clearButton")', 'document.getElementById("refreshButton")'),
  /invalidateInspection\(\)[\s\S]*refreshInspection\(true\)/,
  "clear must invalidate and reload persisted state"
);
assert.match(
  section('document.getElementById("refreshButton")', 'document.getElementById("mobileInspectorToggle")'),
  /refreshRuntime\(false\)[\s\S]*\(state\.activeTab === "dna" \|\| state\.activeTab === "memory"\)[\s\S]*refreshInspection\(true\)/,
  "manual refresh must reload persisted state after health"
);
const evolutionAction = section(
  "async function runEvolutionAction",
  'document.querySelectorAll(".case-button")'
);
const evolutionError = evolutionAction.indexOf("throw new Error(apiErrorText");
const evolutionSuccess = evolutionAction.indexOf("const previous = state.lastResponse", evolutionError + 1);
const evolutionInvalidate = evolutionAction.indexOf("invalidateInspection();", evolutionSuccess);
const evolutionCatch = evolutionAction.indexOf("} catch (error)", evolutionSuccess);
assert.ok(
  evolutionError >= 0
    && evolutionError < evolutionSuccess
    && evolutionSuccess < evolutionInvalidate
    && evolutionInvalidate < evolutionCatch,
  "successful Apply or Rollback must invalidate persisted state after the error branch"
);
const start = html.indexOf("function invalidateInspection()");
const end = html.indexOf("function mutationApplied", start);
assert.ok(start >= 0 && end > start, "inspection lifecycle source must be extractable");

const state = {
  busy: false,
  activeTab: "dna",
  lastResponse: null,
  inspection: null,
  inspectionError: null,
  inspectionLoading: false,
  inspectionRefreshQueued: false,
  inspectionEpoch: 0,
  sessionId: "console-test-session",
  health: { ok: true, version: "test" },
  modelPool: { quality_ready: true }
};
let renderCount = 0;
const requests = [];

function deferred() {
  let resolve;
  const promise = new Promise((done) => { resolve = done; });
  return { promise, resolve };
}

function response(data, ok = true, status = 200) {
  return { ok, status, json: async () => data };
}

function inspectionPayload(generation) {
  return {
    ok: true,
    state: {
      genome_profiles: [{ profile: "general", generation }],
      evolution_live_inference_runs: generation,
      memories: 5,
      runtime_kv_memories: 2,
      runtime_kv_vector_dimensions: [{ dimensions: 64, count: 2 }],
      evolution_live_stored_runtime_kv_memories: 7,
      top_runtime_kv_memories: [{
        id: 42,
        key: "must-not-render",
        vector_dimensions: 64,
        strength: 0.75
      }]
    }
  };
}

function fetchStub(url, options) {
  assert.equal(url, "/v1/inspect");
  const request = deferred();
  requests.push({ ...request, options });
  return request.promise;
}

function apiErrorText(data, status) {
  return data?.error || `HTTP ${status}`;
}

const renderInspectorSource = section("function renderInspector()", "function setStatus");
const renderPanel = { innerHTML: "" };
const renderState = {
  lastResponse: null,
  previousResponse: null,
  activeTab: "memory",
  inspection: inspectionPayload(4),
  inspectionLoading: false,
  inspectionError: null,
  health: {},
  modelPool: {
    workers: [
      { role: "index", quarantine: { retry_after_unix: 1_700_000_060 } },
      { role: "review", quarantine: { retry_after_unix: 1_700_000_120 } }
    ],
    capacity: {}
  },
  behaviorValidation: null
};
const renderInspector = new Function(
  "state",
  "elements",
  "responseMeta",
  "display",
  "metricGroup",
  `${renderInspectorSource}; return renderInspector;`
)(
  renderState,
  { panel: renderPanel, profile: { value: "general" } },
  (value) => value?.norion || {},
  (value) => String(value ?? "-"),
  (title, rows) => `${title}\n${rows.map(([label, value]) => `${label}:${value ?? "-"}`).join("\n")}`
);
renderInspector();
assert.match(renderPanel.innerHTML, /持久 Runtime KV:2/);
assert.match(renderPanel.innerHTML, /Runtime KV 向量维度:64 × 2/);
assert.match(renderPanel.innerHTML, /Runtime KV Top ID:42/);
assert.equal(
  renderPanel.innerHTML.includes("must-not-render"),
  false,
  "persisted Runtime KV keys must remain hidden"
);
renderState.activeTab = "routing";
renderInspector();
assert.match(renderPanel.innerHTML, /隔离 Worker:2 \(index, review\)/);
assert.match(renderPanel.innerHTML, /最早重试时间:/);

const requestCompletionStreamSource = section(
  "async function requestCompletionStream",
  "async function runSingle"
);
const runSingleSource = section("async function runSingle", "async function runComparison");
assert.match(
  runSingleSource,
  /可见续写 \$\{display\(result\.visibleContinuationCharsPerSecond\)\} 字符\/s[\s\S]*\$\{result\.streamedChars\} 字符/,
  "successful streams must render browser-visible content throughput"
);
const streamErrorBytes = new TextEncoder().encode(
  'data: {"choices":[{"delta":{}}],"error":{"message":"runtime exploded"},"norion":{"stream_state":"failed"}}\n\ndata: [DONE]\n\n'
);
let streamRead = false;
const requestCompletionStream = new Function(
  "fetch",
  "completionPayload",
  "apiErrorText",
  `${requestCompletionStreamSource}; return requestCompletionStream;`
)(
  async () => ({
    ok: true,
    status: 200,
    body: { getReader: () => ({
      read: async () => {
        if (streamRead) return { value: undefined, done: true };
        streamRead = true;
        return { value: streamErrorBytes, done: false };
      }
    }) }
  }),
  () => ({}),
  (data, status) => data?.error?.message || `HTTP ${status}`
);
await assert.rejects(
  requestCompletionStream("prompt", [], () => {}),
  /runtime exploded/,
  "SSE error chunks must reject instead of committing an empty answer"
);

async function runSuccessfulStream(sse, clock) {
  const streamBytes = new TextEncoder().encode(sse);
  let read = false;
  const request = new Function(
    "fetch",
    "completionPayload",
    "apiErrorText",
    "performance",
    `${requestCompletionStreamSource}; return requestCompletionStream;`
  )(
    async () => ({
      ok: true,
      status: 200,
      body: { getReader: () => ({
        read: async () => {
          if (read) return { value: undefined, done: true };
          read = true;
          return { value: streamBytes, done: false };
        }
      }) }
    }),
    () => ({}),
    (data, status) => data?.error?.message || `HTTP ${status}`,
    { now: () => clock.shift() }
  );
  const progress = [];
  const result = await request(
    "prompt",
    [],
    (answer, update) => progress.push({ answer, progress: update })
  );
  return { result, progress };
}

const successfulSse =
  'data: {"id":"chatcmpl-test","object":"chat.completion.chunk","created":10,"model":"apple-quality","choices":[{"delta":{"content":"😀"}}]}\n\n'
  + 'data: {"id":"chatcmpl-test","object":"chat.completion.chunk","created":10,"model":"apple-quality","choices":[{"delta":{"content":"好"}}]}\n\n'
  + 'data: {"id":"chatcmpl-test","object":"chat.completion.chunk","created":10,"model":"apple-quality","choices":[{"delta":{},"finish_reason":"stop"}],"norion":{"answer":"😀好","streamed_tokens":2,"runtime_model":"apple-quality"}}\n\n'
  + 'data: [DONE]\n\n';
const { result: streamSuccess, progress: streamProgress } = await runSuccessfulStream(
  successfulSse,
  [100, 300, 1300]
);
assert.equal(streamSuccess.firstContentMs, 200);
assert.equal(streamSuccess.browserElapsedMs, 1200);
assert.equal(streamSuccess.streamedChars, 2);
assert.equal(streamSuccess.visibleContinuationElapsedMs, 1000);
assert.equal(streamSuccess.visibleContinuationCharsPerSecond, 1);
assert.equal(streamProgress.at(-1).answer, "😀好");
assert.equal(streamProgress.at(-1).progress.streamedChars, 2);
assert.equal(streamProgress.at(-1).progress.deltaCount, 2);

const singleContentSse =
  'data: {"id":"chatcmpl-test","object":"chat.completion.chunk","created":10,"model":"apple-quality","choices":[{"delta":{"content":"😀"}}]}\n\n'
  + 'data: {"id":"chatcmpl-test","object":"chat.completion.chunk","created":10,"model":"apple-quality","choices":[{"delta":{},"finish_reason":"stop"}],"norion":{"answer":"😀","streamed_tokens":1,"runtime_model":"apple-quality"}}\n\n'
  + 'data: [DONE]\n\n';
const { result: singleContent } = await runSuccessfulStream(singleContentSse, [100, 300, 1300]);
assert.equal(singleContent.streamedChars, 1);
assert.equal(singleContent.visibleContinuationElapsedMs, 1000);
assert.equal(singleContent.visibleContinuationCharsPerSecond, null);

const { result: zeroDuration } = await runSuccessfulStream(successfulSse, [100, 300, 300]);
assert.equal(zeroDuration.streamedChars, 2);
assert.equal(zeroDuration.visibleContinuationElapsedMs, 0);
assert.equal(zeroDuration.visibleContinuationCharsPerSecond, null);

const missingContentEvidenceSse =
  'data: {"id":"chatcmpl-test","object":"chat.completion.chunk","created":10,"model":"apple-quality","choices":[{"delta":{},"finish_reason":"stop"}],"norion":{"answer":"final-only","streamed_tokens":0,"runtime_model":"apple-quality"}}\n\n'
  + 'data: [DONE]\n\n';
const { result: missingContentEvidence } = await runSuccessfulStream(
  missingContentEvidenceSse,
  [100, 1300]
);
assert.equal(missingContentEvidence.firstContentMs, null);
assert.equal(missingContentEvidence.streamedChars, 0);
assert.equal(missingContentEvidence.visibleContinuationElapsedMs, null);
assert.equal(missingContentEvidence.visibleContinuationCharsPerSecond, null);

const lifecycle = new Function(
  "state",
  "renderInspector",
  "apiErrorText",
  "fetch",
  `${html.slice(start, end)}; return { invalidateInspection, refreshInspection };`
)(state, () => { renderCount += 1; }, apiErrorText, fetchStub);
const flush = () => new Promise((resolve) => setImmediate(resolve));

const initial = lifecycle.refreshInspection();
await flush();
assert.equal(requests.length, 1, "initial DNA load should issue one inspect request");
assert.equal(state.inspectionLoading, true);
assert.deepEqual(JSON.parse(requests[0].options.body), {
  state_gate: false,
  business_gate: false,
  business_cycle_gate: false,
  model_service_gate: false,
  trace_gate: false,
  tenant_id: "local-console",
  workspace_id: "rust-norion",
  session_id: "console-test-session"
});

await lifecycle.refreshInspection();
await flush();
assert.equal(requests.length, 1, "ordinary rerenders must reuse the in-flight request");

await lifecycle.refreshInspection(true);
await lifecycle.refreshInspection(true);
assert.equal(requests.length, 1, "forced refreshes must coalesce while inspect is in flight");
assert.equal(state.inspectionRefreshQueued, true);

requests[0].resolve(response(inspectionPayload(1)));
await initial;
await flush();
assert.equal(requests.length, 2, "coalesced refresh should start exactly one follow-up request");
assert.equal(state.inspection, null, "stale epoch response must not populate the cache");

requests[1].resolve(response(inspectionPayload(2)));
await flush();
await flush();
assert.equal(state.inspection.state.genome_profiles[0].generation, 2);
assert.equal(state.inspectionLoading, false);

await lifecycle.refreshInspection();
await flush();
assert.equal(requests.length, 2, "cached inspection must not refetch without an event");

const healthBeforeFailure = structuredClone(state.health);
const modelPoolBeforeFailure = structuredClone(state.modelPool);
const failed = lifecycle.refreshInspection(true);
await flush();
requests[2].resolve(response({ error: "inspect unavailable" }, false, 503));
await failed;
assert.equal(state.inspection, null);
assert.equal(state.inspectionError, "inspect unavailable");
assert.deepEqual(state.health, healthBeforeFailure, "inspect failure must not alter health state");
assert.deepEqual(
  state.modelPool,
  modelPoolBeforeFailure,
  "inspect failure must not alter model-pool state"
);

const recovered = lifecycle.refreshInspection();
await flush();
requests[3].resolve(response(inspectionPayload(3)));
await recovered;
assert.equal(state.inspection.state.genome_profiles[0].generation, 3);

state.activeTab = "memory";
lifecycle.invalidateInspection();
const memoryLoad = lifecycle.refreshInspection();
await flush();
assert.equal(requests.length, 5, "Memory load should issue one inspect request");
requests[4].resolve(response(inspectionPayload(4)));
await memoryLoad;
assert.equal(state.inspection.state.runtime_kv_memories, 2);

const requestCount = requests.length;
state.busy = true;
await lifecycle.refreshInspection(true);
state.busy = false;
state.activeTab = "routing";
await lifecycle.refreshInspection(true);
state.activeTab = "memory";
state.lastResponse = { ok: true };
await lifecycle.refreshInspection(true);
state.lastResponse = null;
await new Promise((resolve) => setTimeout(resolve, 10));
assert.equal(requests.length, requestCount, "guarded states must not issue inspect requests");
assert.ok(renderCount > 0);

clearTimeout(watchdog);
console.log("operator-console-inspection-lifecycle-ok");
