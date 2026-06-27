# 中文摘要：Reasoning Genome Chain

标题：**Reasoning Genome Chain: A DNA-Inspired Control Layer for Auditable Self-Evolving AI Inference**

中文标题建议：**推理基因链：面向可审计自进化 AI 推理的 DNA 启发控制层**

版本：Technical Report v0.1
日期：2026-06-26
作者：杨浩 / Yang Hao
项目：rust-norion
GitHub：https://github.com/yanghao1143/rust-norion
Gitee：https://gitee.com/babalibaba/rust-norion
Zenodo DOI：https://doi.org/10.5281/zenodo.20901489
OSF：https://osf.io/cybdm/
ScienceDB DOI：https://doi.org/10.57760/sciencedb.41287
OpenI：https://openi.pcl.ac.cn/asd8841315/rust-norion

## 摘要

rust-norion 不是一个新的大模型推理内核，也不是某个模型 API 的简单封装。它关注的是模型外层的控制系统：记忆检索、任务路由、工具调用、反思、证据门禁、回滚和自进化准入。

论文提出 **Reasoning Genome Chain**，一种 DNA 启发的软件控制层抽象。它不重训模型权重，而是把可复用的推理策略表示为有边界、可审计、可测试的 **ReasoningGene**。不同任务 profile 可以选择不同的推理基因链，避免所有任务共享一套不可解释的全局启发式。

系统采用双链架构：

- `express_chain`：运行时可见的控制链，影响 routing、memory retrieval、reflection、tool dispatch、budget posture 等；
- `memory_chain`：append-only 的证据链，保存来源、fitness、drift、验证门禁、拒绝原因和 rollback anchor。

论文还描述 **Gene Scissors**：受控的基因编辑管线，支持 relabel、cut、splice、quarantine、repair、crossover、rollback、regenerate 等操作。所有 durable mutation 默认拒绝，必须经过 trace、test、benchmark、drift、privacy、license、rollback 和 operator approval gates。

这篇报告的定位是技术报告 v0.1，而不是已完成的大规模实验论文。它的贡献在于提出一个 Rust 实现的、可审计的 AI 推理控制层工程框架，并给出当前原型、验证边界和后续研究路线。

新增自动化：仓库现在可以通过 `tools/outreach/generate-publication-update.ps1` 根据最近提交自动生成数据集描述、中文更新稿、英文更新稿和 JSON manifest。它不会自动绕过平台登录、验证码或机构邮箱要求，但能保证 ScienceDB、OSF、GitHub Release、OpenI、CSDN 和社区文章的文案跟着项目更新一起刷新。

## 核心算法

rust-norion 的核心算法不是“训练一个新大模型”，而是 **Evidence-Gated Reasoning Genome Evolution**：证据门禁驱动的推理基因进化算法。它把运行时策略表示成推理基因，把 prompt、memory、KV、trace、ledger 中的片段转成可审计的 GeneSegment，再通过 DnaSplicer、MutDetector、MutFixer 和 DnaEvolutionController 形成只读 mutation plan。

算法主线：

1. 按任务 profile 选择当前 ReasoningGenome 的 `express_chain`；
2. 投影为 `GenomeExpression`，影响 routing、retrieval、reflection、tool dispatch、budget posture 和 validation hints；
3. 只输出脱敏 `ExpressionTrace`，包含 id、计数、delta、gate 和 digest，不写入原始私有 payload；
4. `DnaSplicer.preview` 把 prompt、memory、KV、trace、ledger 片段分类为 exon、intron、variant；
5. `MutDetector` 检测 drift、stale label、privacy risk、schema failure、KV-shape failure、empty range 和 missing hash；
6. `MutFixer` 把 finding 转成 read-only `MutationPlan`，支持 relabel、cut、splice、quarantine、repair、crossover、rollback、regenerate；
7. `DnaEvolutionController` 按 fitness delta、validation evidence、rollback anchor、operator decision 和 writer gate 决定 reject、hold、rollback 或 activation-eligible；
8. `WriterGate` 默认保持 `write_allowed=false`，只有 validation、privacy、rollback、license 和 explicit approval 全部通过后才允许 durable mutation。

这个算法可以公开。应该公开的是流程、状态机、伪代码、脱敏 trace schema 和可复现实验；不公开真实 API key、私有 prompt、原始 trace、`.ndkv`、provider 配额和任何能绕过 writer gate 的操作细节。

## 适合投放的平台

优先级建议：

1. **GitHub Release + Zenodo DOI**：已完成，当前 DOI 为 `10.5281/zenodo.20901489`。
2. **OSF 项目归档**：已完成公开归档，作为论文和补充材料入口。
3. **ScienceDB**：已提交并进入审核/预览状态，可用于国内 DOI/CSTR 引用。
4. **OpenI**：已迁移公开项目，适合国内 AI 开源社区曝光。
5. **ChinaXiv / SinoXiv**：等待作者登录、实名/机构邮箱等平台条件满足后继续。

## 平台短摘要

This technical report introduces the Reasoning Genome Chain, a DNA-inspired software-control abstraction implemented in the open-source rust-norion prototype. Rather than retraining model weights, the approach represents reusable reasoning behavior as auditable strategy records that can influence memory retrieval, routing, reflection, tool dispatch, evidence gates, and rollback. A dual-chain architecture separates runtime expression from append-only provenance, while Gene Scissors provides guarded mutation intents such as relabel, splice, quarantine, rollback, and regenerate. The report frames self-evolving inference as a preview-first, evidence-gated Rust control-layer problem.

## 中文宣传摘要

rust-norion 正在尝试把 AI 推理外层做成一条可审计的“推理基因链”：记忆、路由、反思、工具调用、证据门禁、回滚和自进化都被表示成可测试、可组合、可追踪的 Rust 控制层记录。它不是 AI 壳子，而是面向可验证自进化 AI 系统的底层控制层原型。

## 关键词

Reasoning Genome Chain; Rust; AI inference control layer; self-evolving systems; agent memory; rollback; evidence gates; auditable AI; Gene Scissors; runtime governance

中文关键词：推理基因链；Rust；AI 推理控制层；自进化系统；智能体记忆；回滚；证据门禁；可审计 AI；基因剪刀；运行时治理
