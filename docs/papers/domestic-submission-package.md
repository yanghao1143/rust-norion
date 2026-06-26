# 国内发布包：rust-norion Reasoning Genome Chain Technical Report v0.1

本文件把 rust-norion 的技术报告、代码归档、数据材料和国内预印本发布拆成可执行步骤。目标不是把宣传稿到处重复发布，而是形成一条国内可引用、可检索、可复现的发布链路。

## 当前可上传材料

- 英文论文 Markdown：`docs/papers/reasoning-genome-chain-technical-report-v0.1.md`
- 英文论文 PDF：`docs/papers/reasoning-genome-chain-technical-report-v0.1.pdf`
- 英文论文 HTML：`docs/papers/reasoning-genome-chain-technical-report-v0.1.html`
- 中文摘要：`docs/papers/reasoning-genome-chain-technical-report-v0.1.zh.md`
- 海外投稿包：`docs/papers/submission-package.md`
- 投稿压缩包：`docs/papers/reasoning-genome-chain-submission-package-v0.1.zip`
- 引用元数据：`CITATION.cff`
- Zenodo 元数据：`.zenodo.json`

作者署名：杨浩 / Yang Hao

## 2026-06-26 实际发布状态

- ScienceDB：已提交，列表状态为“待审核”；预览引用显示
  `DOI:10.57760/sciencedb.41287` 和 `CSTR:31253.11.sciencedb.41287`。
- OpenI 启智社区：已迁移公开项目
  `https://openi.pcl.ac.cn/asd8841315/rust-norion`，简介和标签已补齐。
- ChinaXiv：停在中国科技云通行证登录，需作者本人登录后继续投稿。
- SinoXiv：已进入国家预印本平台账号，但投稿权限要求机构邮箱激活或机构邮箱申请；正式投稿前还需确认账号实名与作者署名“杨浩 / Yang Hao”一致。

## 国内路线总览

推荐顺序：

1. **Gitee/GitHub Release 固化版本**：生成源码压缩包和明确版本号。
2. **ScienceDB 归档代码和配套材料**：上传 Release 源码包、PDF、复现实验说明、测试/日志样本，申请平台支持的永久标识。
3. **ChinaXiv 发布预印本**：上传论文 PDF，正文引用 ScienceDB 归档链接和 GitHub/Gitee 仓库。
4. **SinoXiv 作为第二预印本入口**：如平台政策允许，上传同一技术报告或中文增强版。
5. **OpenI 启智社区导入或申请项目**：作为国内 AI 开源社区曝光和孵化入口，不把它当作 DOI 平台。
6. **后续期刊/会议**：等补足 benchmark、ablation、复现数据后，再投软件工程、计算机系统、AI 工程类期刊或会议。

## 平台定位校准

| 平台 | 更准确定位 | rust-norion 用法 | 注意事项 |
| --- | --- | --- | --- |
| ScienceDB | 科学数据长期共享、出版和开放获取平台 | 固化源码包、论文 PDF、实验脚本、日志和复现材料 | 是否 DOI/CSTR、字段和审核要求以提交页为准 |
| ChinaXiv | 中科院科技论文预发布平台 | 发布技术报告预印本，争取国内理工科可检索曝光 | 预印本不是同行评议正式论文 |
| SinoXiv | 国家预印本平台 | 作为第二国内预印本入口，扩大检索面 | 需要实名注册和平台初审 |
| OpenI 启智社区 | AI 开源社区、代码数据托管和项目孵化入口 | 导入/托管项目，申请社区展示、新闻稿或孵化流程 | 不是简单一键 DOI 替代，项目加入需按社区流程 |
| Gitee | 国内代码托管和访问加速 | 镜像 GitHub，发布 Release 包，便于国内访问 | 学术引用仍建议绑定 ScienceDB/预印本链接 |

## ScienceDB 上传包建议

建议数据集或资源标题：

`rust-norion Reasoning Genome Chain Technical Report v0.1: Source, Manuscript, and Reproducibility Materials`

中文标题：

`rust-norion 推理基因链技术报告 v0.1：源码、论文与复现材料`

建议上传内容：

- GitHub/Gitee Release 源码包；
- `reasoning-genome-chain-technical-report-v0.1.pdf`；
- `reasoning-genome-chain-technical-report-v0.1.md`；
- `reasoning-genome-chain-technical-report-v0.1.zh.md`；
- `submission-package.md`；
- `CITATION.cff`；
- `.zenodo.json`；
- 后续可追加 benchmark 输出、trace 样例、CI 验证摘要、性能日志。

ScienceDB 简介文案：

> This archive preserves the v0.1 technical report and reproducibility materials for rust-norion, an open-source Rust prototype for auditable AI inference control. The report introduces the Reasoning Genome Chain, a DNA-inspired software-control abstraction that represents reusable reasoning behavior as typed strategy records, separates runtime expression from append-only provenance, and gates self-evolving mutations through trace, tests, benchmark evidence, rollback anchors, privacy checks, and operator approval.

中文简介：

> 本归档保存 rust-norion v0.1 技术报告及复现材料。rust-norion 是一个开源 Rust AI 推理控制层原型，提出 DNA 启发的 Reasoning Genome Chain，将可复用推理策略表示为有类型、可审计的策略记录，并通过 express_chain / memory_chain 双链架构区分运行时表达与证据来源。系统默认 preview-first，任何自进化 mutation 都必须经过 trace、test、benchmark、rollback、privacy 和 operator approval gates。

关键词：

`Rust; AI inference control; Reasoning Genome Chain; auditable AI; self-evolving systems; agent memory; rollback; evidence gates; Gene Scissors`

每日或每次版本迭代补充数据集描述时，固定写清四件事：版本变化、上传文件、可复现材料、DNA/RGC 最新成果。不要上传真实 API key、未脱敏日志或私有 trace。

## ChinaXiv 上传文案

标题：

`Reasoning Genome Chain: A DNA-Inspired Control Layer for Auditable Self-Evolving AI Inference`

中文标题：

`推理基因链：面向可审计自进化 AI 推理的 DNA 启发控制层`

作者：

`Yang Hao (杨浩)`

摘要：

> Large language model applications increasingly depend on control logic outside the model weights: memory retrieval, routing, tool use, reflection, evaluation, rollback, and operator approval. In many prototypes this control logic is implicit, scattered across prompts, scripts, logs, and ad hoc agent state. This technical report introduces the Reasoning Genome Chain, a DNA-inspired software-control abstraction implemented in the open-source rust-norion prototype. Rather than retraining model weights, the approach represents reusable reasoning behavior as bounded, typed, auditable strategy records called reasoning genes. A task profile selects an express chain that can influence runtime routing, retrieval, reflection, budget posture, tool dispatch, and validation gates, while a separate memory chain preserves provenance, fitness evidence, rejection reasons, rollback anchors, and privacy-safe digests. Gene Scissors provides a guarded mutation pipeline for relabel, cut, splice, quarantine, repair, crossover, rollback, and regenerate operations. Durable mutation is denied by default and must pass trace, test, benchmark, drift, privacy, license, rollback, and operator-approval gates before admission. The contribution is a concrete engineering frame for building auditable self-evolving inference control layers in Rust.

中文摘要：

> 大语言模型应用越来越依赖模型权重之外的控制逻辑：记忆检索、任务路由、工具调用、反思、评估、回滚和人工批准。许多原型系统将这些控制逻辑隐含在 prompt、脚本、日志和临时 agent 状态中，导致系统难以审计和复现。本文提出 Reasoning Genome Chain，一种在 rust-norion 开源原型中实现的 DNA 启发软件控制层抽象。该方法不重训模型权重，而是将可复用推理行为表示为有边界、有类型、可审计的策略记录，即 reasoning genes。任务 profile 选择 express_chain 影响运行时路由、检索、反思、预算姿态、工具调度和验证门禁；memory_chain 则保存来源、fitness 证据、拒绝原因、rollback anchor 和隐私安全摘要。Gene Scissors 提供 relabel、cut、splice、quarantine、repair、crossover、rollback、regenerate 等受控 mutation 管线。所有 durable mutation 默认拒绝，必须经过 trace、test、benchmark、drift、privacy、license、rollback 和 operator approval gates。本文贡献是一个用 Rust 构建可审计自进化 AI 推理控制层的工程框架。

学科建议：

- 计算机科学与技术
- 软件工程
- 人工智能
- 计算机系统结构
- 开源软件 / AI 系统工程，如果平台提供该类目

## SinoXiv 上传文案

如果 ChinaXiv 已经发布，SinoXiv 的描述应明确这是同一技术报告的预印本版本，避免造成重复发表误解。

简介：

> This is a technical-report preprint for the rust-norion open-source project. The manuscript proposes the Reasoning Genome Chain, a DNA-inspired control-layer abstraction for auditable self-evolving AI inference. It focuses on software architecture, safety gates, rollback, provenance, and Rust implementation surfaces rather than model-weight training.

## OpenI 启智社区适配文案

项目名称：

`rust-norion`

一句话介绍：

`基于 Rust 的可审计 AI 推理控制层原型，提出 DNA 启发的 Reasoning Genome Chain，用于记忆、路由、反思、证据门禁、回滚和自进化治理。`

项目简介：

> rust-norion 是一个开源 Rust AI 推理控制层原型，不是生产级大模型推理内核，也不是某个模型 API 的简单封装。项目探索如何把模型外层的控制系统工程化：记忆检索、任务路由、工具调用、反思、证据门禁、回滚、自进化准入和审计追踪。核心概念 Reasoning Genome Chain 将可复用推理策略表示为有类型、可审计、可回滚的 reasoning genes，并通过 Gene Scissors 管线进行 preview-first 的受控 mutation。

适配方向：

- 国产 AI 开源基础设施；
- Rust 高可靠系统；
- AI Agent memory / routing / reflection；
- 可审计自进化系统；
- 本地优先 AI 控制层。

建议附件：

- 技术报告 PDF；
- GitHub / Gitee 链接；
- 贡献者专区；
- Reasoning Genome Chain 架构文档；
- 短视频或封面图作为社区宣传材料。

## 国内外双轨引用策略

第一阶段：

- GitHub/Gitee Release 固化代码版本；
- ScienceDB 上传 Release 包和论文 PDF；
- ChinaXiv 上传技术报告；
- OpenI 做国内 AI 开源展示。

第二阶段：

- Zenodo 给 GitHub Release 发 DOI；
- OSF 归档论文和补充材料；
- TechRxiv 发布英文预印本。

第三阶段：

- 把 DOI、CSTR 或平台永久链接补回 README、CITATION.cff、论文正文；
- outreach registry 中把此前因“需要 paper artifact”而 defer 的 AI memory、agent evolution、LLM systems、benchmark 论文列表重新打开；
- 准备 v0.2 benchmark 报告，再投期刊或会议。

## 风险和边界

- 不要把预印本说成同行评议正式论文。
- 不要宣称 ScienceDB/OpenI/ChinaXiv 已经分配 DOI 或 CSTR，除非提交后平台页面真实显示。
- 不要夸大 v0.1 的实证结论；当前论文是技术报告和原型架构，不是 SOTA benchmark 论文。
- 不要上传真实 API key、私有日志、未脱敏 trace、模型权重或 raw `.ndkv`。
- 如果平台要求单位、邮箱、ORCID、基金信息，需要作者本人在网页中填写。
