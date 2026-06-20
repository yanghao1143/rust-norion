# Noiron 经验库卫生检查

## 目的

本地模型不会因为聊天自动改权重。它看起来“越用越聪明”或“越用越蠢”，主要取决于持久化经验、检索上下文和业务反馈是不是干净。

如果经验库把不同任务的对话混在一起，例如 Rust 代码问题混入 SSH、GitLab、MR、Token 或命令日志，后续检索就可能把这些内容重新塞进模型上下文。表现就是：用户问一个简单问题，模型突然输出旧任务日志或无关运维内容。

## 只检查，不启动模型

这个命令只读取本地状态文件，不会启动 Gemma 12B，也不会占用 GPU 推理：

```powershell
cd D:\rust-norion
cargo run -- --experience-hygiene --experience-hygiene-limit 20
```

如果还想顺便看完整状态摘要，可以跑：

```powershell
cargo run -- --inspect-state --inspect-limit 5
```

重点看这两段输出：

```text
experience_hygiene_findings=4 experience_hygiene_quarantine_candidates=4
```

以及底部：

```text
experience_hygiene: findings=4 quarantine_candidates=4
  id=861 severity=quarantine_candidate reason=cross_task_shell_transcript markers=...
```

现在检查命令还会同时打印长经验索引质量：

```text
experience_index: total_records=863 compacted_records=194 noisy_records=1 max_noise_penalty=0.180000 listed=1
  id=861 reason=unstructured_long_transcript compacted=true noise_penalty=0.180000 prompt_chars=...
```

字段含义：

- `experience_hygiene_findings`：发现的疑似脏经验数量。
- `experience_hygiene_quarantine_candidates`：建议隔离的经验数量。
- `id`：经验记录 ID。
- `reason=cross_task_shell_transcript`：跨任务会话污染，通常是聊天记录里混入命令日志或其他任务上下文。
- `markers`：触发规则的证据，例如 `ssh_connect_timeout`、`gitlab_local`、`merge_requests`、`bash_command`。
- `prompt` / `lesson`：压缩后的样本预览，用来确认是不是误报。
- `experience_index.listed` / `id`：缺少 gist/结构化摘要的长文本样本，用来定位是哪条经验让检索索引变脏。

## 当前保护

检索层已经加了保守保护：如果一条经验同时满足以下条件，普通聊天不会再自动召回它：

- 像 `Conversation transcript` / `user:` / `assistant:` 这样的整段会话记录；
- 包含 SSH、GitLab、MR、Token 或命令日志等跨任务标记；
- 当前用户问题本身不是在询问这些运维/GitLab/SSH 内容。

这能减少“问 Rust for 循环却召回 GitLab 命令日志”的情况。

## 为什么不自动删除

污染记录可能仍有排障价值，所以当前实现只做报告和检索隔离，不直接删除 `.ndkv` 数据。

真正清理前建议先跑检查，确认 ID 和预览内容：

```powershell
cargo run -- --inspect-state --inspect-limit 20
```

确认后再做单独的隔离/重写工具，避免误删仍有价值的经验。

## 隔离候选，默认只演练

先 dry-run，确认会移动哪些记录：

```powershell
cargo run -- --experience-hygiene-quarantine --experience-hygiene-limit 20
```

确认无误后，显式加 `--experience-hygiene-apply` 才会写文件。执行时会先备份原始经验库，再把污染候选移动到旁路 quarantine 文件：

```powershell
cargo run -- --experience-hygiene-quarantine --experience-hygiene-apply --experience-hygiene-limit 20
```

也可以指定备份和隔离文件位置：

```powershell
cargo run -- --experience-hygiene-quarantine --experience-hygiene-apply --experience-hygiene-backup-path .\noiron-experience.backup.ndkv --experience-hygiene-quarantine-path .\noiron-experience.quarantine.ndkv
```

没有 `--experience-hygiene-apply` 时不会修改主经验库。

## 用 gate 阻止脏经验进入联调

要让自检在发现污染候选时直接失败，可以加 gate：

```powershell
cargo run -- --inspect-state --inspect-gate --inspect-max-experience-hygiene-quarantine-candidates 0 --inspect-max-experience-repairable-legacy-metadata-lessons 0 --inspect-max-experience-repairable-index-records 0 --inspect-max-experience-repair-projected-legacy-metadata-lessons 0 --inspect-max-experience-repair-skipped-missing-clean-gist 0
```

也可以同时要求长经验索引没有无结构噪声：

```powershell
cargo run -- --inspect-state --inspect-gate --inspect-max-experience-hygiene-quarantine-candidates 0 --inspect-max-experience-repairable-legacy-metadata-lessons 0 --inspect-max-experience-repairable-index-records 0 --inspect-max-experience-repair-projected-legacy-metadata-lessons 0 --inspect-max-experience-repair-skipped-missing-clean-gist 0 --inspect-max-experience-index-noisy-records 0 --inspect-max-experience-index-noise-penalty 0
```

这仍然只读状态文件，不会启动模型，也不会修改 `.ndkv`。如果输出里出现：

```text
state_inspection_gate_failure: experience_hygiene_quarantine_candidate_count 4 above maximum 0
```

说明当前经验库还有需要隔离或重写的污染候选。

如果输出里出现：

```text
state_inspection_gate_failure: experience_repairable_legacy_metadata_lesson_count 828 above maximum 0
```

说明经验库里还有可自动迁移的旧格式 `accepted_pattern/rejected_pattern` lesson。可以先用 `--experience-repair` 做 dry-run 预览，不加 `--experience-repair-apply` 不会写主经验库。

如果输出里出现：

```text
state_inspection_gate_failure: experience_repair_projected_legacy_metadata_lesson_count 32 above maximum 0
```

说明即使自动迁移 repairable 经验后，仍有旧格式 lesson 需要隔离、补 clean gist，或人工重写。

如果输出里出现：

```text
state_inspection_gate_failure: experience_repair_skipped_missing_clean_gist_count 28 above maximum 0
```

说明有旧格式 lesson 因缺少可复用 clean gist，不能安全自动迁移。

如果输出里出现：

```text
state_inspection_gate_failure: experience_index_noisy_record_count 1 above maximum 0
```

说明经验库里仍有缺少 gist/结构化摘要的长文本记录，检索层会降权它，但联调前仍建议补摘要或隔离。

## 预览某个问题会召回哪些经验

发送真实 prompt 之前，可以先只读预览检索结果。这个命令不会启动 Gemma，也不会写 `.ndkv`：

```powershell
cargo run -- --experience-retrieval --experience-retrieval-limit 5 --profile coding "帮我用rust输出一段for循环代码"
```

重点看：

- `matches`：当前 prompt 会召回多少条经验。
- `skipped_cross_task_pollution`：有多少条跨任务 transcript 被卫生规则跳过。
- `id` / `score` / `lesson`：实际会进入上下文候选的经验 ID、分数和摘要。

如果 `skipped_cross_task_pollution=4`，说明那几条 SSH/GitLab/MR 污染记录没有进入本次召回；如果 `matches` 里仍出现旧的无关 transcript，就继续补索引摘要或隔离策略。

## 机器卡顿时先停模型

如果本地推理把电脑卡住，先停外围联调栈：

```powershell
cd D:\rust-norion
.\tools\smartsteam-forge\stop-forge.cmd
```

确认端口没有监听：

```powershell
Get-NetTCPConnection -LocalPort 7878,8787,8686 -ErrorAction SilentlyContinue
```

确认没有模型相关进程：

```powershell
Get-Process | Where-Object { $_.ProcessName -match 'mistral|gemma|rustgpt|smartsteam|norion' }
```

后续再启动测试 UI 或 Gemma 前，先用 `--inspect-state` 看经验库是否干净。

## 服务端状态字段

模型服务的 `/v1/state` 也会暴露经验库卫生字段，前端或测试 UI 可以在开始聊天前先检查：

```powershell
curl.exe -s http://127.0.0.1:7878/v1/state
```

重点看：

- `state.experience_hygiene_findings`
- `state.experience_hygiene_quarantine_candidates`
- `state.experience_hygiene_clean`
- `state.experience_hygiene_samples`
- `state.experience_index_compacted_records`
- `state.experience_index_noisy_records`
- `state.experience_index_max_noise_penalty`
- `state.experience_index_samples`

如果 `experience_hygiene_clean=false` 或 `experience_hygiene_quarantine_candidates>0`，建议先暂停真实模型联调，执行 dry-run 隔离确认。

`experience_index_*` 字段用于观察长经验索引质量：被压缩的记录越多，说明经验库里长文本越多；`noisy_records` 和 `max_noise_penalty` 则表示有多少长文本缺少 gist/结构化摘要，检索时已经被降权。
底层索引投影会优先使用 `clean_gist`；如果某条经验还没有 clean gist，只会写入有界的 `prompt_excerpt`/`lesson_excerpt` fallback，并在索引元数据里标记 `content_basis=raw_fallback`、`content_truncated=true/false`、`prompt_chars` 和 `lesson_chars`，避免单条超长 transcript 直接污染向量索引。
检索排序也会读取这些元数据：`raw_fallback` 会被温和降权，`content_truncated=true` 会再次降权；进入上下文注入前，候选还会携带 `raw_fallback_index_content` 和 `truncated_index_content` 风险原因。即使该候选最终被 admit/summarize，这些原因也会保留在 `memory_context_injection` 的 `reason_codes` / `detail_codes` 中，`accepted_risk` 会统计本次有多少条已接纳上下文仍携带风险信号，方便后续 gate 或前端解释为什么某条经验没有被优先使用、以及判断脏索引是否已经进入模型上下文。
适配器的 read-only shadow plan 也复用同一批 `MemoryIndexDocument` 投影来生成上下文候选，所以 dry-run、迁移预检和真实检索看到的是同一套索引质量信号。
`experience_index_samples` 会列出样本 ID、原因、噪声惩罚和 prompt/lesson 预览，方便定位具体是哪条经验影响检索。

## 服务端健康检查字段

`/health` 不需要拿 engine 推理锁，适合前端在发送消息前轮询。它会在经验库文件存在时只读扫描卫生状态；如果文件不存在，只返回 `checked=false`，不会创建新的 `.ndkv` 文件：

```powershell
curl.exe -s http://127.0.0.1:7878/health
```

重点看：

- `experience_hygiene.checked`
- `experience_hygiene.clean`
- `experience_hygiene.findings`
- `experience_hygiene.quarantine_candidates`
- `experience_hygiene.error`
- `readiness_warnings`

如果 `experience_hygiene.clean=false`，`readiness_warnings` 会包含 `experience_hygiene` 提示；前端可以显示“先隔离脏经验再聊天”。

## 服务端卫生检查和隔离接口

模型服务现在提供单独的经验库卫生接口，方便 SmartSteam Forge 或其他前端在聊天前检查，不需要调用推理接口：

```powershell
curl.exe -s http://127.0.0.1:7878/v1/experience-hygiene
```

返回重点字段：

- `checked`：是否成功读取经验库。
- `report.clean`：是否没有隔离候选。
- `report.quarantine_candidates`：建议隔离的记录数。
- `index_report.compacted_records`：长经验索引被压缩的记录数。
- `index_report.noisy_records`：缺少 gist/结构化摘要、检索时会降权的长文本记录数。
- `index_report.max_noise_penalty`：当前最大检索噪声惩罚。
- `index_report.listed_findings`：长文本索引噪声样本，包含经验 ID、原因、噪声惩罚和预览。
- `quarantine_plan.candidate_ids`：候选经验 ID。
- `quarantine_plan.listed_findings`：候选样本预览。

服务端隔离接口默认只 dry-run，不写主经验库：

```powershell
curl.exe -s -X POST http://127.0.0.1:7878/v1/experience-hygiene/quarantine -H "Content-Type: application/json" -d "{\"limit\":20}"
```

也可以通过服务端预览某个 prompt 的经验召回，不触发模型推理：

```powershell
curl.exe -s -X POST http://127.0.0.1:7878/v1/experience-retrieval -H "Content-Type: application/json" -d "{\"prompt\":\"帮我用rust输出一段for循环代码\",\"profile\":\"coding\",\"limit\":5}"
```

返回重点字段：

- `retrieval.match_count`
- `retrieval.skipped_cross_task_pollution`
- `retrieval.matches[].experience_id`
- `retrieval.matches[].score`
- `retrieval.matches[].lesson_preview`

只有显式传入 `apply=true` 才会写入。应用时会先备份原始经验库，再把污染候选写到 quarantine 文件，最后重写主经验库：

```powershell
curl.exe -s -X POST http://127.0.0.1:7878/v1/experience-hygiene/quarantine -H "Content-Type: application/json" -d "{\"apply\":true,\"limit\":20}"
```

也可以指定文件位置：

```powershell
curl.exe -s -X POST http://127.0.0.1:7878/v1/experience-hygiene/quarantine -H "Content-Type: application/json" -d "{\"apply\":true,\"limit\":20,\"backup_path\":\"D:\\rust-norion\\noiron-experience.backup.ndkv\",\"quarantine_path\":\"D:\\rust-norion\\noiron-experience.quarantine.ndkv\"}"
```

建议前端策略：

- 启动后先轮询 `/health`。
- 如果 `experience_hygiene.clean=false`，展示警告并禁用重模型聊天。
- 用户可以先调用 `/v1/experience-retrieval` 预览当前 prompt 会召回哪些经验，不触发推理。
- 用户确认后调用 `/v1/experience-hygiene` 展示候选。
- 真正清理必须调用 `/v1/experience-hygiene/quarantine` 且请求体包含 `"apply":true`。
