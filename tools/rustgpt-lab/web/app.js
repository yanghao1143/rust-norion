(() => {
    'use strict';

    if (window.__rustgptLabAppStarted) return;
    window.__rustgptLabAppStarted = true;

    const form = document.getElementById('chatForm');
    const promptEl = document.getElementById('prompt');
    const messages = document.getElementById('messages');
    const send = document.getElementById('send');
    const cancel = document.getElementById('cancel');
    const statusLine = document.getElementById('statusLine');
    const backendLine = document.getElementById('backendLine');
    const modelPoolLine = document.getElementById('modelPoolLine');
    const modelPoolAdviceLine = document.getElementById('modelPoolAdviceLine');
    const contextLine = document.getElementById('contextLine');
    const endpointEl = document.getElementById('endpoint');
    const maxTokensEl = document.getElementById('maxTokens');
    const contextLimitEl = document.getElementById('contextLimit');
    const clearContext = document.getElementById('clearContext');
    const followOutput = document.getElementById('followOutput');
    const businessControls = document.getElementById('businessControls');
    const rustCheckCode = document.getElementById('rustCheckCode');
    const mainEl = document.querySelector('main');
    let backendAvailable = false;
    let backendBusy = false;
    let modelReady = false;
    let submitPending = false;
    let inFlight = false;
    let activeController = null;
    let activeReader = null;
    let activeAssistant = null;
    let activeProgressMeta = null;
    let cancelRequested = false;
    let autoScroll = followOutput.checked;
    let suppressScrollSync = false;
    let scrollSyncTimer = null;
    let userScrollIntent = false;
    let conversation = [];
    let conversationEpoch = 0;
    let backendHealthInFlight = null;
    let backendHealthLastStarted = 0;
    let modelPoolInFlight = null;
    let modelPoolLastStarted = 0;
    let modelPoolAdviceInFlight = null;
    let modelPoolAdviceLastStarted = 0;
    const DEFAULT_CONTEXT_MESSAGES = 64;
    const MIN_CONTEXT_MESSAGES = 2;
    const MAX_CONTEXT_MESSAGES = 256;
    const GENERATION_MAX_TOKENS = 262144;
    const DEFAULT_MAX_TOKENS = GENERATION_MAX_TOKENS;
    const STREAM_HEALTH_REFRESH_INTERVAL_MS = 1500;
    const MODEL_POOL_REFRESH_INTERVAL_MS = 5000;
    const AUTO_SCROLL_BOTTOM_THRESHOLD_PX = 64;
    const AUTO_SCROLL_SYNC_SUPPRESS_MS = 120;

    function numericControlValue(input, fallback, min, max) {
      const parsed = Number.parseInt(input.value, 10);
      const value = Number.isFinite(parsed) ? parsed : fallback;
      const clamped = Math.min(max, Math.max(min, value));
      if (String(clamped) !== input.value) input.value = String(clamped);
      return clamped;
    }

    function contextMessageLimit() {
      return numericControlValue(contextLimitEl, DEFAULT_CONTEXT_MESSAGES, MIN_CONTEXT_MESSAGES, MAX_CONTEXT_MESSAGES);
    }

    function generationMaxTokens() {
      return numericControlValue(maxTokensEl, DEFAULT_MAX_TOKENS, 1, GENERATION_MAX_TOKENS);
    }

    function recentConversationForSend(limit) {
      const keep = Math.max(0, limit - 1);
      return keep === 0 ? [] : conversation.slice(-keep);
    }

    function updateContextLine(count = conversation.length, options = {}) {
      const limit = contextMessageLimit();
      const trimConversation = options.trimConversation !== false;
      if (trimConversation && conversation.length > limit) conversation = conversation.slice(-limit);
      const displayedCount = Math.min(count, limit);
      contextLine.textContent = `上下文：${displayedCount}/${limit} 条短会话消息，生成预算 max_tokens=${generationMaxTokens()}，临时浏览器会话`;
    }

    function updateEndpointControls() {
      const business = endpointEl.value === 'business-cycle';
      businessControls.classList.toggle('active', business);
      rustCheckCode.style.display = business ? 'block' : 'none';
    }
    endpointEl.addEventListener('change', updateEndpointControls);
    maxTokensEl.addEventListener('change', () => updateContextLine());
    contextLimitEl.addEventListener('change', () => updateContextLine());
    updateEndpointControls();
    updateContextLine();
    clearContext.addEventListener('click', () => {
      conversation = [];
      conversationEpoch += 1;
      updateContextLine();
      addMeta('context cleared');
      setStatus('上下文已清空');
      promptEl.focus();
    });

    function addMessage(kind, text) {
      const node = document.createElement('div');
      node.className = `msg ${kind}`;
      node.textContent = text;
      messages.appendChild(node);
      scrollToBottom();
      return node;
    }

    function addStreamMessage(kind, text) {
      const node = document.createElement('div');
      node.className = `msg ${kind}`;
      node.textContent = text;
      appendBeforeActiveAssistant(node);
      return node;
    }

    function addMeta(text) {
      const node = document.createElement('div');
      node.className = 'meta';
      node.textContent = text;
      messages.appendChild(node);
      scrollToBottom();
      return node;
    }

    function addStreamMeta(text) {
      const node = document.createElement('div');
      node.className = 'meta';
      node.textContent = text;
      appendBeforeActiveAssistant(node);
      return node;
    }

    function upsertStreamProgressMeta(event, data) {
      const text = `${event}: ${data}`;
      if (activeProgressMeta && activeProgressMeta.parentNode === messages) {
        activeProgressMeta.textContent = text;
        if (activeAssistant && activeAssistant.parentNode === messages) {
          messages.insertBefore(activeProgressMeta, activeAssistant);
        }
        scrollToBottom();
        return activeProgressMeta;
      }
      activeProgressMeta = addStreamMeta(text);
      activeProgressMeta.classList.add('progress');
      return activeProgressMeta;
    }

    function appendBeforeActiveAssistant(node) {
      if (activeAssistant && activeAssistant.parentNode === messages) {
        messages.insertBefore(node, activeAssistant);
      } else {
        messages.appendChild(node);
      }
      scrollToBottom();
    }

    function setStatus(text) {
      statusLine.textContent = text;
    }

    function isNearBottom() {
      return mainEl.scrollHeight - mainEl.scrollTop - mainEl.clientHeight <= AUTO_SCROLL_BOTTOM_THRESHOLD_PX;
    }

    function setAutoScroll(value, scrollNow = false) {
      autoScroll = value;
      followOutput.checked = value;
      if (scrollNow && autoScroll) scrollToBottom(true);
    }

    function scrollToBottom(force = false) {
      if (!force && !autoScroll) return;
      suppressScrollSync = true;
      if (scrollSyncTimer) clearTimeout(scrollSyncTimer);
      jumpToBottom();
      requestAnimationFrame(() => {
        jumpToBottom();
        requestAnimationFrame(() => {
          jumpToBottom();
          scrollSyncTimer = setTimeout(() => {
            suppressScrollSync = false;
            scrollSyncTimer = null;
          }, AUTO_SCROLL_SYNC_SUPPRESS_MS);
        });
      });
    }

    function jumpToBottom() {
      mainEl.scrollTo({ top: mainEl.scrollHeight, behavior: 'auto' });
    }

    function noteUserScrollIntent() {
      userScrollIntent = true;
      suppressScrollSync = false;
    }

    function syncAutoScrollFromViewport() {
      if (suppressScrollSync && !userScrollIntent) return;
      setAutoScroll(isNearBottom());
      userScrollIntent = false;
    }

    function setAssistantStreamState(node, state) {
      if (!node) return;
      node.classList.toggle('streaming', state === 'streaming');
      node.classList.toggle('interrupted', state === 'interrupted');
    }

    function markStreamInterrupted(node, reason) {
      setAssistantStreamState(node, 'interrupted');
      setStatus(`流式连接中断：${reason}；已保留已收到内容，未写入上下文。`);
      addStreamMeta(`stream interrupted: ${reason}; partial output kept, context not updated`);
      scrollToBottom();
    }

    function setBackendStatus(text, kind = '') {
      backendLine.className = `backendline ${kind}`.trim();
      backendLine.textContent = text;
    }

    function setModelPoolStatus(text, kind = '') {
      modelPoolLine.className = `poolline ${kind}`.trim();
      modelPoolLine.textContent = text;
    }

    function setModelPoolAdvice(text, kind = '') {
      modelPoolAdviceLine.className = `adviceline ${kind}`.trim();
      modelPoolAdviceLine.textContent = text;
    }

    function updateSendAvailability() {
      send.disabled = submitPending || inFlight || !backendAvailable || !modelReady || backendBusy;
      cancel.hidden = !inFlight;
      cancel.disabled = !inFlight || !activeController || cancelRequested;
      promptEl.disabled = inFlight;
      rustCheckCode.disabled = inFlight;
      if (inFlight) send.textContent = '发送中';
      else if (submitPending) send.textContent = '检查中';
      else if (!backendAvailable) send.textContent = '后端离线';
      else if (!modelReady) send.textContent = '预检失败';
      else if (backendBusy) send.textContent = '后端忙';
      else send.textContent = '发送';
    }

    function healthModelReady(health) {
      if (!health) return false;
      if (health.readiness_ok === false || health.safe_device_ok === false) return false;
      if (!experienceHealthReady(health)) return false;
      const requiresGemma = health.runtime_mode === 'gemma-http' && Boolean(health.gemma_runtime_server);
      return !(requiresGemma && health.gemma_runtime_reachable === false);
    }

    function experienceHealthReady(health) {
      const hygiene = health && health.experience_hygiene;
      if (!hygiene) return true;
      if (hygiene.clean === false) return false;
      if (positiveNumber(hygiene.quarantine_candidates)) return false;
      if (positiveNumber(hygiene.repairable_legacy_metadata_lessons)) return false;
      if (positiveNumber(hygiene.repairable_index_records)) return false;
      const index = hygiene.index;
      return !(index && (index.retrieval_ready === false || index.risk_level === 'blocked'));
    }

    function runtimeLabel(health) {
      if (!health || !health.runtime_mode) return 'rust-norion';
      if (health.runtime_mode === 'built-in') return 'built-in 安全后端';
      if (health.runtime_mode === 'gemma-http') return 'Gemma 12B';
      return health.runtime_mode;
    }

    function shortPath(path) {
      if (!path) return 'unknown';
      const parts = String(path).split(/[\\/]/);
      return parts.slice(Math.max(0, parts.length - 3)).join('\\');
    }

    function joinFailures(values) {
      if (!Array.isArray(values) || values.length === 0) return '';
      return `，failures ${values.join('; ')}`;
    }

    function experienceHealthDetail(hygiene) {
      if (!hygiene) return '';
      const hygieneState = hygiene.clean === false
        ? '需清理'
        : hygiene.checked === false
          ? '未检查'
          : 'OK';
      const index = hygiene.index;
      const debt = [];
      if (positiveNumber(hygiene.quarantine_candidates)) debt.push(`quarantine ${hygiene.quarantine_candidates}`);
      if (positiveNumber(hygiene.repairable_legacy_metadata_lessons)) debt.push(`repair legacy ${hygiene.repairable_legacy_metadata_lessons}`);
      if (positiveNumber(hygiene.repairable_index_records)) debt.push(`repair index ${hygiene.repairable_index_records}`);
      const debtState = debt.length ? `，${debt.join(' / ')}` : '';
      const indexBlocked = index && (index.retrieval_ready === false || index.risk_level === 'blocked');
      const indexState = index
        ? `，索引 ${indexBlocked ? '阻断' : index.risk_level || 'unknown'} score ${index.quality_score ?? '?'} noisy ${index.noisy_records ?? '?'} dup ${index.duplicate_outputs ?? '?'}`
        : '';
      return `，经验 ${hygieneState} ${shortPath(hygiene.experience_file)}${debtState}${indexState}`;
    }

    function gemmaRuntimeDetail(health) {
      if (!health || !health.gemma_runtime_server) return '';
      const parts = [];
      if (health.gemma_runtime_model) parts.push(health.gemma_runtime_model);
      if (health.gemma_runtime_context_window) {
        const train = health.gemma_runtime_train_context_window
          ? `/${health.gemma_runtime_train_context_window}`
          : '';
        parts.push(`n_ctx ${health.gemma_runtime_context_window}${train}`);
      }
      if (health.gemma_runtime_metadata_error) {
        parts.push(`metadata ${health.gemma_runtime_metadata_error}`);
      }
      return parts.length ? ` (${parts.join(', ')})` : '';
    }

    function refreshBackendHealth() {
      if (backendHealthInFlight) return backendHealthInFlight;
      backendHealthLastStarted = Date.now();
      backendHealthInFlight = fetchBackendHealth().finally(() => {
        backendHealthInFlight = null;
      });
      return backendHealthInFlight;
    }

    function refreshBackendHealthThrottled() {
      if (backendHealthInFlight) return backendHealthInFlight;
      if (Date.now() - backendHealthLastStarted < STREAM_HEALTH_REFRESH_INTERVAL_MS) {
        return Promise.resolve(null);
      }
      return refreshBackendHealth();
    }

    function refreshModelPoolStatus() {
      if (modelPoolInFlight) return modelPoolInFlight;
      modelPoolLastStarted = Date.now();
      modelPoolInFlight = fetchModelPoolStatus().finally(() => {
        modelPoolInFlight = null;
      });
      return modelPoolInFlight;
    }

    function refreshModelPoolStatusThrottled() {
      if (modelPoolInFlight) return modelPoolInFlight;
      if (Date.now() - modelPoolLastStarted < MODEL_POOL_REFRESH_INTERVAL_MS) {
        return Promise.resolve(null);
      }
      return refreshModelPoolStatus();
    }

    function refreshModelPoolAdvice() {
      if (modelPoolAdviceInFlight) return modelPoolAdviceInFlight;
      modelPoolAdviceLastStarted = Date.now();
      modelPoolAdviceInFlight = fetchModelPoolAdvice().finally(() => {
        modelPoolAdviceInFlight = null;
      });
      return modelPoolAdviceInFlight;
    }

    function refreshModelPoolAdviceThrottled() {
      if (modelPoolAdviceInFlight) return modelPoolAdviceInFlight;
      if (Date.now() - modelPoolAdviceLastStarted < MODEL_POOL_REFRESH_INTERVAL_MS) {
        return Promise.resolve(null);
      }
      return refreshModelPoolAdvice();
    }

    async function fetchBackendHealth() {
      try {
        const response = await fetch('/api/backend-health', { cache: 'no-store' });
        const health = await response.json();
        if (!health.ok) {
          backendAvailable = false;
          modelReady = false;
          backendBusy = false;
          setBackendStatus(`后端状态：不可用 ${health.error || ''}`.trim(), 'error');
          updateSendAvailability();
          return health;
        }
        const activeCount = health.active_engine_requests ?? 0;
        const seen = health.requests_seen ?? 0;
        const runtime = health.runtime_mode ? `，runtime ${health.runtime_mode}` : '';
        const gemmaReachable = health.gemma_runtime_reachable;
        const gemmaState = gemmaReachable === true
          ? '在线'
          : gemmaReachable === false
            ? '未启动'
            : '未知';
        const gemma = health.gemma_runtime_server ? `，Gemma ${gemmaState} ${health.gemma_runtime_server}` : '';
        const gemmaDetail = gemmaRuntimeDetail(health);
        const last = health.last_inference
          ? `，上次 ${health.last_inference.endpoint || '推理'} ${health.last_inference.elapsed_ms ?? '?'}ms/${health.last_inference.runtime_token_count ?? '?'} tokens`
          : '';
        const activeRequest = Array.isArray(health.active_requests) && health.active_requests.length
          ? health.active_requests[0]
          : null;
        const activeDetail = activeRequest
          ? `，当前 #${activeRequest.request_id ?? '?'} ${activeRequest.endpoint ?? '?'} 已运行 ${activeRequest.elapsed_ms ?? '?'}ms：${activeRequest.prompt_preview ?? ''}`
          : '';
        const hygiene = health.experience_hygiene;
        const experience = experienceHealthDetail(hygiene);
        const readiness = health.readiness_ok === false
          ? `，readiness=false${joinFailures(health.readiness_failures)}`
          : '';
        const safeDevice = health.safe_device_ok === false
          ? `，safe-device=false${joinFailures(health.safe_device_failures)}`
          : '';
        backendAvailable = true;
        modelReady = healthModelReady(health);
        backendBusy = Boolean(health.engine_busy);
        if (health.engine_busy) {
          setBackendStatus(`后端状态：忙，活跃推理 ${activeCount}${activeDetail}，已处理请求 ${seen}${runtime}${gemma}${gemmaDetail}${experience}${readiness}${safeDevice}${last}`, 'busy');
        } else if (!modelReady) {
          setBackendStatus(`后端状态：预检失败，已处理请求 ${seen}${runtime}${gemma}${gemmaDetail}${experience}${readiness}${safeDevice}${last}`, 'error');
        } else {
          setBackendStatus(`后端状态：空闲，已处理请求 ${seen}${runtime}${gemma}${gemmaDetail}${experience}${last}`);
        }
        updateSendAvailability();
        return health;
      } catch (error) {
        backendAvailable = false;
        modelReady = false;
        backendBusy = false;
        setBackendStatus(`后端状态：检查失败 ${error.message}`, 'error');
        updateSendAvailability();
        return null;
      }
    }

    async function fetchModelPoolStatus() {
      try {
        const response = await fetch('/api/model-pool-status', { cache: 'no-store' });
        const pool = await response.json();
        if (!pool.ok) {
          setModelPoolStatus(`模型池：不可用 ${pool.error || ''}`.trim(), 'error');
          setModelPoolAdvice('模型池建议：先恢复模型池状态接口，再判断是否扩容', 'error');
          return pool;
        }
        setModelPoolStatus(formatModelPoolStatus(pool), modelPoolStatusKind(pool));
        setModelPoolAdvice(formatModelPoolAdvice(pool), modelPoolAdviceKind(pool));
        refreshModelPoolAdviceThrottled();
        return pool;
      } catch (error) {
        setModelPoolStatus(`模型池：检查失败 ${error.message}`, 'error');
        setModelPoolAdvice('模型池建议：后端状态不可读，暂时不要增加 worker', 'error');
        return null;
      }
    }

    async function fetchModelPoolAdvice() {
      try {
        const response = await fetch('/api/model-pool-advice', { cache: 'no-store' });
        const advice = await response.json();
        if (!advice.ok) {
          setModelPoolAdvice(`模型池建议：${advice.error || '不可用'}`, 'error');
          return advice;
        }
        const detail = helperRoleContractDetail(advice);
        setModelPoolAdvice(advice.advice ? `${advice.advice}${detail}` : formatModelPoolAdvice(advice), advice.kind || '');
        return advice;
      } catch (error) {
        return null;
      }
    }

    function formatModelPoolStatus(pool) {
      const workers = Array.isArray(pool.workers) ? pool.workers : [];
      const total = numberOr(pool.worker_count, workers.length);
      const healthy = numberOr(
        pool.healthy_worker_count,
        workers.filter((worker) => worker.ready === true || worker.role_ready === true).length
      );
      const routeMetrics = pool.route_metrics
        ? `，${formatPoolMetrics(pool.route_metrics)}`
        : '';
      const minContext = pool.min_context_tokens
        ? `，min_ctx ${pool.min_context_tokens}`
        : '';
      const dispatch = pool.launch_allowed === false
        ? `，调度阻断 ${pool.launch_block_reason || pool.reason || 'unknown'}`
        : '，可调度';
      const capacity = pool.capacity
        ? `，${formatPoolCapacity(pool.capacity)}`
        : '';
      const workerSummary = workers.length
        ? `，${workers.map(formatPoolWorker).join(' | ')}`
        : '';
      return `模型池：${healthy}/${total} healthy${dispatch}${minContext}${capacity}${routeMetrics}${workerSummary}`;
    }

    function modelPoolStatusKind(pool) {
      if (pool.launch_allowed === false || numberOr(pool.healthy_worker_count, 0) === 0) return 'error';
      if (metricNumber(pool.route_metrics, 'in_flight') > 0) return 'busy';
      const workers = Array.isArray(pool.workers) ? pool.workers : [];
      return workers.some((worker) => metricNumber(worker, 'in_flight') > 0) ? 'busy' : '';
    }

    function formatModelPoolAdvice(pool) {
      const facts = modelPoolFacts(pool);
      const context = facts.qualityContext && facts.qualityRequiredContext
        ? `ctx ${facts.qualityContext}/${facts.qualityRequiredContext}`
        : facts.qualityContext
          ? `ctx ${facts.qualityContext}`
          : 'ctx unknown';
      const suffix = '；不要多开 12B，优先一主多小';
      const helperDetail = helperRoleContractDetail(pool);
      if (facts.qualityReady === false) {
        return `模型池建议：先恢复 quality 12B(8686)，${context}${helperDetail}${suffix}`;
      }
      if (facts.qualityContextSufficient === false) {
        return `模型池建议：重启 quality 并提高上下文窗口，${context}${helperDetail}${suffix}`;
      }
      if (facts.qualityCpuFallback || facts.qualityZeroGpuLayers) {
        return `模型池建议：先修 Metal/GPU 或 gpu_layers，再加小模型${helperDetail}${suffix}`;
      }
      if (facts.capacityRecommendation === 'restore_quality_gate_first') {
        return `模型池建议：先恢复 quality gate，再考虑 summary/review/index${helperDetail}${suffix}`;
      }
      if (facts.extraQuality12BDetected) {
        return `模型池建议：检测到多个 quality 12B，先停掉多余大模型，只保留 1 个 12B 主力，再挂 summary/review/index/test-gate 小模型${helperDetail}${suffix}`;
      }
      if (!facts.hasSummary) {
        return `模型池建议：quality 可用，先加 summary 小模型，${context}${helperDetail}${suffix}`;
      }
      if (!facts.hasReview || !facts.hasIndex) {
        return `模型池建议：summary 已可用，短 smoke 后补 review 或 index 小模型${helperDetail}${suffix}`;
      }
      if (!facts.hasTestGate) {
        return `模型池建议：review/index 已可用，确认内存压力正常后再加 test-gate${helperDetail}${suffix}`;
      }
      return `模型池建议：helper 池已成形，可用 /pool-call 与 evolution-loop helper 阶段联调${helperDetail}${suffix}`;
    }

    function helperRoleContractDetail(pool) {
      const advice = pool.advice || {};
      const missing = Array.isArray(pool.missing_helper_roles)
        ? pool.missing_helper_roles
        : Array.isArray(advice.missing_helper_roles)
          ? advice.missing_helper_roles
          : [];
      const launchOrder = Array.isArray(pool.recommended_launch_order)
        ? pool.recommended_launch_order
        : Array.isArray(advice.recommended_launch_order)
          ? advice.recommended_launch_order
          : [];
      const launchDetail = launchOrder.length > 0 ? `；启动顺序 ${launchOrder.join('/')}` : '';
      if (missing.length > 0) {
        return `；缺 helper ${missing.join('/')}${launchDetail}`;
      }
      const expected = Array.isArray(pool.expected_helper_roles)
        ? pool.expected_helper_roles
        : Array.isArray(advice.expected_helper_roles)
          ? advice.expected_helper_roles
          : [];
      return expected.length > 0
        ? `；helper 目标 ${expected.join('/')}${launchDetail}`
        : launchDetail;
    }

    function modelPoolAdviceKind(pool) {
      const facts = modelPoolFacts(pool);
      if (
        facts.qualityReady === false ||
        facts.qualityContextSufficient === false ||
        facts.qualityCpuFallback ||
        facts.qualityZeroGpuLayers ||
        facts.capacityRecommendation === 'restore_quality_gate_first' ||
        facts.extraQuality12BDetected
      ) {
        return 'error';
      }
      if (!facts.hasSummary || !facts.hasReview || !facts.hasIndex || !facts.hasTestGate) {
        return 'busy';
      }
      return '';
    }

    function modelPoolFacts(pool) {
      const workers = Array.isArray(pool.workers) ? pool.workers : [];
      const quality = workers.find((worker) => worker.role === 'quality') || null;
      const capacity = pool.capacity || {};
      const helperRoles = pool.helper_roles || {};
      const qualityWorkerCount = numberOr(
        pool.quality_worker_count,
        workers.filter((worker) => worker.role === 'quality').length
      );
      return {
        qualityReady: boolOr(pool.quality_ready, workerReady(quality)),
        qualityContextSufficient: boolOr(pool.quality_context_sufficient, null),
        qualityContext: pool.quality_context_tokens || quality?.context_window || quality?.default_context_tokens,
        qualityRequiredContext: pool.quality_context_required_tokens,
        qualityCpuFallback: workerLooksCpuBound(quality),
        qualityZeroGpuLayers: metricNumber(quality, 'gpu_layers') === 0 && quality?.gpu_layers !== undefined && quality?.gpu_layers !== null,
        capacityRecommendation: capacity.recommendation || pool.capacity_recommendation || '',
        extraQuality12BDetected: pool.extra_quality_12b_detected === true || qualityWorkerCount > 1,
        qualityWorkerCount,
        helperWorkerCount: numberOr(
          pool.helper_worker_count,
          workers.filter((worker) => ['summary', 'review', 'index', 'test-gate'].includes(worker.role)).length
        ),
        hasSummary: helperRoles.summary === true || workers.some((worker) => worker.role === 'summary' && workerReady(worker)),
        hasReview: helperRoles.review === true || workers.some((worker) => worker.role === 'review' && workerReady(worker)),
        hasIndex: helperRoles.index === true || workers.some((worker) => worker.role === 'index' && workerReady(worker)),
        hasTestGate: helperRoles.test_gate === true || helperRoles['test-gate'] === true || workers.some((worker) => worker.role === 'test-gate' && workerReady(worker)),
      };
    }

    function workerReady(worker) {
      if (!worker) return null;
      return worker.ready === true || worker.role_ready === true || worker.status === 'healthy' || worker.status === 'ready';
    }

    function workerLooksCpuBound(worker) {
      if (!worker) return false;
      return ['cpu', 'cpu-vector'].includes(worker.runtime_device)
        || ['cpu', 'none'].includes(worker.runtime_accelerator);
    }

    function boolOr(value, fallback) {
      return typeof value === 'boolean' ? value : fallback;
    }

    function formatPoolMetrics(metrics) {
      const parts = [
        `route ${numberOr(metrics.route_count, 0)}`,
        `selected ${numberOr(metrics.selected_count, 0)}`,
        `blocked ${numberOr(metrics.blocked_count, 0)}`,
        `busy ${numberOr(metrics.in_flight, 0)}`
      ];
      if (metrics.avg_latency_ms !== null && metrics.avg_latency_ms !== undefined) {
        parts.push(`avg ${metrics.avg_latency_ms}ms`);
      }
      return parts.join('/');
    }

    function formatPoolCapacity(capacity) {
      const allowed = capacity.expansion_allowed === true ? '可扩容' : '先别扩容';
      const recommendation = capacity.recommendation || 'unknown';
      const helpers = `${numberOr(capacity.healthy_helper_worker_count, 0)}/${numberOr(capacity.helper_worker_count, 0)} helpers`;
      const runtime = [
        `metal ${numberOr(capacity.metal_worker_count, 0)}`,
        `cpu ${numberOr(capacity.cpu_worker_count, 0)}`,
        `unknown ${numberOr(capacity.unknown_runtime_worker_count, 0)}`,
        `gpu0 ${numberOr(capacity.zero_gpu_layer_worker_count, 0)}`
      ].join('/');
      return `容量 ${allowed} ${recommendation} ${helpers} ${runtime}`;
    }

    function formatPoolWorker(worker) {
      const role = worker.role || '?';
      const status = worker.ready === true || worker.role_ready === true
        ? 'ready'
        : worker.status || worker.role_block_reason || 'down';
      const busy = metricNumber(worker, 'in_flight') > 0
        ? `/busy ${metricNumber(worker, 'in_flight')}`
        : '';
      const latency = worker.avg_latency_ms !== null && worker.avg_latency_ms !== undefined
        ? `/avg ${worker.avg_latency_ms}ms`
        : '';
      const failures = metricNumber(worker, 'failure_count') > 0
        ? `/fail ${metricNumber(worker, 'failure_count')}`
        : '';
      return `${role}:${status}${busy}${latency}${failures}`;
    }

    function metricNumber(source, key) {
      if (!source || source[key] === null || source[key] === undefined) return 0;
      const value = Number(source[key]);
      return Number.isFinite(value) ? value : 0;
    }

    function numberOr(value, fallback) {
      const parsed = Number(value);
      return Number.isFinite(parsed) ? parsed : fallback;
    }

    function positiveNumber(value) {
      const parsed = Number(value);
      return Number.isFinite(parsed) && parsed > 0;
    }

    refreshBackendHealth();
    refreshModelPoolStatus();
    refreshModelPoolAdvice();
    setInterval(refreshBackendHealth, 5000);
    setInterval(refreshModelPoolStatus, MODEL_POOL_REFRESH_INTERVAL_MS);
    setInterval(refreshModelPoolAdvice, MODEL_POOL_REFRESH_INTERVAL_MS);

    followOutput.addEventListener('change', () => {
      setAutoScroll(followOutput.checked, true);
    });

    mainEl.addEventListener('wheel', noteUserScrollIntent, { passive: true });
    mainEl.addEventListener('touchstart', noteUserScrollIntent, { passive: true });
    mainEl.addEventListener('pointerdown', noteUserScrollIntent, { passive: true });
    mainEl.addEventListener('scroll', syncAutoScrollFromViewport, { passive: true });

    cancel.addEventListener('click', () => {
      if (!activeController) return;
      cancelRequested = true;
      cancel.disabled = true;
      setStatus('正在取消当前请求...');
      addStreamMeta('cancel requested by user');
      activeController.abort();
      if (activeReader) activeReader.cancel().catch(() => {});
    });

    function parseSse(buffer, onEvent) {
      let boundary;
      while ((boundary = nextSseBoundary(buffer)) !== null) {
        const frame = buffer.slice(0, boundary.index);
        buffer = buffer.slice(boundary.index + boundary.length);
        let event = 'message';
        const data = [];
        let sawSseField = false;
        for (const normalized of frame.split(/\r\n|\n|\r/)) {
          if (normalized.startsWith('event:')) {
            sawSseField = true;
            event = parseSseData(normalized.slice(6));
          } else if (normalized === 'event') {
            sawSseField = true;
            event = '';
          }
          if (normalized.startsWith('data:')) {
            sawSseField = true;
            data.push(parseSseData(normalized.slice(5)));
          } else if (normalized === 'data') {
            sawSseField = true;
            data.push('');
          }
        }
        if (!sawSseField) continue;
        if (event.length === 0) event = 'message';
        onEvent(event, data.join('\n'));
      }
      return buffer;
    }

    function parseSseData(value) {
      return value.startsWith(' ') ? value.slice(1) : value;
    }

    function nextSseBoundary(buffer) {
      const boundaries = [
        { index: buffer.indexOf('\n\n'), length: 2 },
        { index: buffer.indexOf('\r\r'), length: 2 },
        { index: buffer.indexOf('\r\n\r\n'), length: 4 }
      ].filter((boundary) => boundary.index >= 0);
      if (boundaries.length === 0) return null;
      return boundaries.sort((left, right) => left.index - right.index)[0];
    }

    form.addEventListener('submit', async (event) => {
      event.preventDefault();
      if (submitPending || inFlight) return;
      const prompt = promptEl.value.trim();
      if (!prompt) return;
      submitPending = true;
      updateSendAvailability();
      const health = await refreshBackendHealth();
      if (!health || !health.ok || health.engine_busy || !healthModelReady(health)) {
        submitPending = false;
        updateSendAvailability();
        if (health && !healthModelReady(health)) {
          setStatus('后端预检未通过；查看状态栏或运行 status-gemma-lab.cmd / status-built-in-lab.cmd。');
          addMeta('blocked: backend readiness, safe-device, experience hygiene, or Gemma runtime failed');
          return;
        }
        setStatus('后端正忙或不可用，等待当前 Gemma 12B 请求结束后再发送。');
        addMeta('blocked: backend is busy or unavailable');
        return;
      }

      addMessage('user', prompt);
      const conversationEpochAtSend = conversationEpoch;
      const contextLimit = contextMessageLimit();
      const maxTokens = generationMaxTokens();
      const outgoingMessages = [
        ...recentConversationForSend(contextLimit),
        { role: 'user', content: prompt }
      ];
      const payload = {
        prompt,
        messages: outgoingMessages,
        profile: document.getElementById('profile').value,
        output: document.getElementById('outputMode').value,
        endpoint: endpointEl.value,
        max_tokens: maxTokens,
        feedback_amount: document.getElementById('feedbackAmount').value,
        self_improve: document.getElementById('selfImprove').checked,
        rust_check_code: rustCheckCode.value.trim()
      };
      updateContextLine(outgoingMessages.length, { trimConversation: false });
      addMessage('raw', `request:\n${formatRequestPreview(payload)}`);
      const assistant = addMessage('assistant', '');
      setAssistantStreamState(assistant, 'streaming');
      activeAssistant = assistant;
      activeProgressMeta = null;
      submitPending = false;
      inFlight = true;
      updateSendAvailability();
      setStatus(`已发送，发送 ${outgoingMessages.length} 条消息，等待 ${runtimeLabel(health)} 推理...`);
      promptEl.value = '';

      let sawDone = false;
      let sawFinal = false;
      let streamHadError = false;
      let shouldRestoreDraft = false;
      try {
        const controller = new AbortController();
        activeController = controller;
        cancelRequested = false;
        updateSendAvailability();
        const response = await fetch('/api/chat-stream', {
          method: 'POST',
          headers: { 'content-type': 'application/json; charset=utf-8' },
          body: JSON.stringify(payload),
          signal: controller.signal
        });
        if (!response.ok || !response.body) throw new Error(`HTTP ${response.status}`);

        const reader = response.body.getReader();
        activeReader = reader;
        const decoder = new TextDecoder();
        let buffer = '';
        const handleStreamEvent = (event, data) => {
          if (event === 'delta') {
            assistant.textContent += data;
            setStatus('正在接收回答...');
            scrollToBottom();
          }
          else if (event === 'raw' && payload.output !== 'raw') addStreamMessage('raw', `raw:\n${data}`);
          else if (event === 'enhanced' && payload.output !== 'enhanced') addStreamMessage('raw', `enhanced:\n${data}`);
          else if (event === 'request') addStreamMessage('raw', `request from proxy:\n${data}`);
          else if (event === 'status' || event === 'heartbeat') {
            setStatus(data);
            upsertStreamProgressMeta(event, data);
          }
          else if (event === 'meta' || event === 'stage') {
            setStatus(data);
            addStreamMeta(`${event}: ${data}`);
          }
          else if (event === 'final') {
            sawFinal = true;
            try {
              const finalPayload = JSON.parse(data);
              const finalAnswer = finalPayload.answer || finalPayload.generate?.answer;
              if (finalAnswer) {
                assistant.textContent = finalAnswer;
              }
              const elapsed = finalPayload.elapsed_ms ?? finalPayload.generate?.elapsed_ms ?? '?';
              const runtimeTokens = finalPayload.runtime_token_count ?? finalPayload.generate?.runtime_token_count ?? '?';
              setStatus(`完成：${elapsed}ms，runtime tokens ${runtimeTokens}`);
              scrollToBottom();
            } catch {
              addStreamMeta(`final: ${data}`);
            }
          }
          else if (event === 'error') {
            streamHadError = true;
            setStatus(`错误：${data}`);
            setAssistantStreamState(assistant, 'interrupted');
            addStreamMeta(`error: ${data}`);
          }
          else if (event === 'done') {
            sawDone = true;
            if (!streamHadError) {
              setAssistantStreamState(assistant, '');
              setStatus('完成');
            }
          }
        };

        for (;;) {
          const { value, done } = await reader.read();
          if (done) {
            buffer += decoder.decode();
            buffer = parseSse(buffer, handleStreamEvent);
            break;
          }
          buffer += decoder.decode(value, { stream: true });
          buffer = parseSse(buffer, handleStreamEvent);
          refreshBackendHealthThrottled();
          refreshModelPoolStatusThrottled();
          refreshModelPoolAdviceThrottled();
          scrollToBottom();
        }
        const hasIncompleteSseBuffer = buffer.trim().length > 0;
        if (cancelRequested) {
          streamHadError = true;
          shouldRestoreDraft = true;
          setStatus('已取消当前请求');
          setAssistantStreamState(assistant, 'interrupted');
          updateContextLine(conversation.length, { trimConversation: false });
        } else if (streamHadError) {
          shouldRestoreDraft = true;
          setAssistantStreamState(assistant, 'interrupted');
          updateContextLine(conversation.length, { trimConversation: false });
        } else if (!sawDone || hasIncompleteSseBuffer) {
          streamHadError = true;
          shouldRestoreDraft = true;
          const reason = hasIncompleteSseBuffer ? 'incomplete SSE frame before EOF' : 'missing done event before EOF';
          markStreamInterrupted(assistant, reason);
          updateContextLine(conversation.length, { trimConversation: false });
        } else if (!streamHadError && assistant.textContent.trim()) {
          setAssistantStreamState(assistant, '');
          if (conversationEpoch === conversationEpochAtSend) {
            const currentContextLimit = contextMessageLimit();
            conversation = [
              ...conversation,
              { role: 'user', content: prompt },
              { role: 'assistant', content: assistant.textContent.trim() }
            ].slice(-currentContextLimit);
          } else {
            addStreamMeta('context cleared during stream; completed turn not added to conversation');
          }
          updateContextLine();
        } else {
          setAssistantStreamState(assistant, '');
          updateContextLine();
        }
      } catch (error) {
        streamHadError = true;
        shouldRestoreDraft = true;
        if (cancelRequested || error.name === 'AbortError') {
          setStatus('已取消当前请求');
          setAssistantStreamState(assistant, 'interrupted');
          addStreamMeta('cancelled');
        } else {
          markStreamInterrupted(assistant, error.message || 'request failed');
          addStreamMeta(`请求失败: ${error.message}`);
        }
        updateContextLine(conversation.length, { trimConversation: false });
      } finally {
        activeReader = null;
        activeController = null;
        activeAssistant = null;
        activeProgressMeta = null;
        cancelRequested = false;
        submitPending = false;
        inFlight = false;
        if (shouldRestoreDraft && !promptEl.value.trim()) {
          promptEl.value = prompt;
        }
        updateSendAvailability();
        promptEl.focus();
        refreshBackendHealth();
        refreshModelPoolStatus();
        refreshModelPoolAdvice();
      }
    });

    function formatRequestPreview(payload) {
      const lines = [
        `endpoint=${payload.endpoint} output=${payload.output} profile=${payload.profile} max_tokens=${payload.max_tokens}`,
        `prompt=${previewText(payload.prompt)}`,
        `messages=${payload.messages.length}`
      ];
      payload.messages.forEach((message, index) => {
        lines.push(`${index + 1}. ${message.role}: ${previewText(message.content)}`);
      });
      return lines.join('\n');
    }

    function previewText(text, maxChars = 160) {
      const normalized = String(text || '')
        .split(/\r?\n/)
        .map((line) => line.trim())
        .filter(Boolean)
        .join(' / ');
      if (normalized.length <= maxChars) return normalized;
      return `${normalized.slice(0, Math.max(0, maxChars - 3))}...`;
    }

    promptEl.addEventListener('keydown', handleComposerKeydown);
    rustCheckCode.addEventListener('keydown', handleComposerKeydown);

    function handleComposerKeydown(event) {
      if (!isPlainEnterSubmit(event)) return;
      event.preventDefault();
      if (!send.disabled) form.requestSubmit();
    }

    function isPlainEnterSubmit(event) {
      return event.key === 'Enter'
        && !event.shiftKey
        && !event.ctrlKey
        && !event.altKey
        && !event.metaKey
        && !event.isComposing
        && event.keyCode !== 229
        && !event.repeat;
    }
})();

