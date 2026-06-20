# RustGPT 前端流式测试实验室

## RustGPT 是什么

[bitswired/rustgpt](https://github.com/bitswired/rustgpt) 是一个 ChatGPT 风格的 Web UI 示例。它的 README 说明该项目使用 Rust 服务端、Axum、HTMX、SQLite、SSE 和 Tera，目标是用 Rust 体验类似 ChatGPT 的 Web 应用形态。该仓库页面也标注许可证为 AGPL-3.0。

对 rust-norion 来说，RustGPT 的价值不是模型推理内核，而是产品形态参考：

- ChatGPT 风格会话界面；
- 服务端驱动 HTML 或轻量前端；
- SSE 流式体验；
- 持久化会话和消息的思路；
- Rust Web 服务组织方式。

## 本项目如何分离耦合

我们没有复制 RustGPT 源码，也没有把它作为依赖接进主程序。当前实现放在：

```text
tools/rustgpt-lab/
```

它是一个独立 Cargo 项目，只做测试工具：

```text
浏览器
  -> rustgpt-lab:8787
  -> rust-norion:7878
  -> Gemma / Noiron 后端
```

这样做有三个好处：

- 不修改 `src/**`，不影响主程序编译和运行边界；
- 不把 AGPL 源码引入 rust-norion 核心；
- 以后可以替换为 Axum/HTMX/Tera 版本，而不污染模型服务。

当前 `tools/rustgpt-lab/src/` 也按职责拆分，避免测试代码继续堆在单个入口文件里：

- `app.rs`：路由和请求生命周期；
- `backend.rs`：代理调用 `rust-norion:7878`；
- `request.rs`：前端请求解析；
- `sse.rs`：SSE 输出；
- `http.rs`：最小 HTTP 工具；
- `chunk.rs`：回答分片；
- `json.rs`：无依赖 JSON 工具；
- `config.rs`：启动参数；
- `status.rs`：长时间 Gemma 推理等待文案和错误提示。

## 启动方式

### 安全 built-in 路径：不启动 Gemma

先用 built-in 后端验证 Web Lab 前后端分离、SSE 代理和 `/health` 状态栏。这条路径不启动 Gemma 12B，也不需要 `mistralrs`。默认使用隔离状态目录 `target\manual-web-lab-service\forge-state`，避免改到项目根目录的 `noiron-*.ndkv`。

推荐一条命令启动 built-in 后端和 Web Lab：

```powershell
cd D:\rust-norion
.\tools\rustgpt-lab\start-built-in-lab.cmd
```

只做静态检查，不启动、不写 `.ndkv`：

```powershell
.\tools\rustgpt-lab\start-built-in-lab.cmd -CheckOnly
```

查看或停止 built-in Web Lab：

```powershell
.\tools\rustgpt-lab\status-built-in-lab.cmd
.\tools\rustgpt-lab\stop-built-in-lab.cmd -DryRun
.\tools\rustgpt-lab\stop-built-in-lab.cmd
```

### 真实 Gemma 路径：先 CheckOnly

真实 Gemma 12B 联调前先跑只读 CheckOnly。它会显示 StateDir、RAM/VRAM、端口、backend health 和经验库安全提示；不会启动 Gemma，也不会写 `.ndkv`。

```powershell
cd D:\rust-norion
cargo run -- `
  --gemma-model-service-smoke `
  --gemma-smoke-check-only `
  --gemma-local-snapshot "D:\hf-cache\hub\models--google--gemma-4-12B-it\snapshots\5926caa4ec0cac5cbfadaf4077420520de1d5205"
```

CheckOnly 通过后，再用脚本启动真实链路：

```powershell
cd D:\rust-norion
.\tools\rustgpt-lab\scripts\start-gemma-lab.ps1 `
  -StateDir target\manual-gemma-service\lab-state
```

如果 CheckOnly 提示项目经验库 dirty，应先使用隔离状态目录，或者在获得显式授权后再清理经验库。只有明确决定使用项目根目录状态时，才使用 `start-gemma-lab.ps1 -UseProjectState`。
如果 7878/8787 已经被别的服务占用，真实启动脚本会读取 `/health` 并确认它们分别是 Gemma 模式 `rust-norion` 和指向该后端的 Web Lab；确认失败会直接退出，避免把 prompt 发到错误后端。

结束真实 Gemma 联调时，先 dry-run 再停止：

```powershell
.\tools\rustgpt-lab\scripts\stop-gemma-lab.ps1 -DryRun
.\tools\rustgpt-lab\scripts\stop-gemma-lab.ps1
```

打开浏览器：

```text
http://127.0.0.1:8787/
```

## 接口

### `GET /`

返回中文测试 UI。支持：

- 中文输入；
- `raw Gemma` / `enhanced Noiron` 输出切换；
- `/v1/chat` / `/v1/generate` 切换；
- `/v1/business-cycle` 业务联调模式；
- profile 切换；
- 流式状态、心跳、回答分片显示。
- 顶部后端状态栏，显示 `engine_busy`、活跃推理数和已处理请求数。

### `POST /api/chat-stream`

请求示例：

```json
{
  "prompt": "请用中文说明当前 Gemma 联调状态。",
  "profile": "coding",
  "output": "raw",
  "endpoint": "chat"
}
```

响应类型：

```text
text/event-stream
```

事件类型：

- `status`: 代理状态；
- `heartbeat`: 等待后端期间的心跳；
- `meta`: 后端运行信息；
- `raw`: 原始模型答案；
- `enhanced`: Noiron 增强答案；
- `delta`: 显示给用户的回答分片；
- `done`: 结束；
- `error`: 错误。

选择 `/v1/business-cycle` 时，代理会额外转发 `feedback_amount`、`self_improve` 和可选 `rust_check_code`，并通过 `meta` 事件显示 `business_cycle_passed`、`feedback_applied`、`rust_check_passed`、`self_improve_passed` 等业务联调结果。

### `GET /api/backend-health`

代理 `rust-norion` 的 `/health`，返回后端是否可用、是否忙碌、活跃推理数和已处理请求数。页面顶部会定时调用它，所以 Gemma 12B 长时间推理时也能看出后端仍然活着。Web Lab 会把 `experience_hygiene.clean=false`、`quarantine_candidates>0`、`repairable_legacy_metadata_lessons>0`、`repairable_index_records>0`、索引 `retrieval_ready=false` 或索引 `risk_level=blocked` 都视为发送前门禁失败；先用 SmartSteam Forge `/doctor`、`/hygiene dry-run`、`/repair dry-run` 或 `/audit` 做只读检查。

## 当前流式能力

`rustgpt-lab` 会优先调用 `rust-norion` 的原生流式接口：

- `/v1/chat-stream`
- `/v1/generate-stream`
- `/v1/business-cycle-stream`

主服务在 Gemma HTTP runtime 下会请求 mistralrs/OpenAI `stream:true`，解析 SSE delta，并通过 service-level SSE 继续透传给 Web Lab。结束时主服务再发送 `final` 事件，包含完整 JSON、Noiron 反思、记忆写入、runtime token 统计等字段。

如果某个 stream 路由不可用，外围代理会回退到非 stream 路由，并把最终 answer 切成小片段显示；这种回退只用于兼容，不代表当前首选路径。

## 许可证注意事项

RustGPT 上游是 AGPL-3.0。AGPL 对网络服务场景有强 copyleft 要求：如果修改并通过网络提供服务，通常需要向用户提供对应源代码。

因此当前策略是：

- 只参考产品形态和公开 README 信息；
- 不复制 RustGPT 源码、模板、CSS 或资产；
- 不把 RustGPT 作为依赖接入主程序；
- 在外围测试工具中用自写实现模拟 ChatGPT 风格流式体验。

这不是法律意见；正式产品化前应再做许可证审查。
