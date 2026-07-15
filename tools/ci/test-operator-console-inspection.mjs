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
  /renderInspector\(\);[\s\S]*if \(loadInspection && state\.activeTab === "dna" && !state\.lastResponse && !state\.busy\) \{[\s\S]*refreshInspection\(\)/,
  "runtime startup must connect the default DNA view to persisted inspection"
);
assert.match(
  section('document.querySelectorAll(".tab")', 'document.getElementById("composer")'),
  /reenteringDna[\s\S]*refreshInspection\(true\)/,
  "DNA re-entry must force a persisted-state refresh"
);
assert.match(
  section('document.getElementById("clearButton")', 'document.getElementById("refreshButton")'),
  /invalidateInspection\(\)[\s\S]*refreshInspection\(true\)/,
  "clear must invalidate and reload persisted state"
);
assert.match(
  section('document.getElementById("refreshButton")', 'document.getElementById("mobileInspectorToggle")'),
  /refreshRuntime\(false\)[\s\S]*refreshInspection\(true\)/,
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
      evolution_live_inference_runs: generation
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

const requestCompletionStreamSource = section(
  "async function requestCompletionStream",
  "async function runSingle"
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

const requestCount = requests.length;
state.busy = true;
await lifecycle.refreshInspection(true);
state.busy = false;
state.activeTab = "routing";
await lifecycle.refreshInspection(true);
state.activeTab = "dna";
state.lastResponse = { ok: true };
await lifecycle.refreshInspection(true);
state.lastResponse = null;
await new Promise((resolve) => setTimeout(resolve, 10));
assert.equal(requests.length, requestCount, "guarded states must not issue inspect requests");
assert.ok(renderCount > 0);

clearTimeout(watchdog);
console.log("operator-console-inspection-lifecycle-ok");
