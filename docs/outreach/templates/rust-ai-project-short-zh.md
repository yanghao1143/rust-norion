rust-norion 是一个开源 Rust AI 推理控制层原型，关注路由、记忆、反思、runtime 边界、证据门禁、回滚和可审计自进化。

它不是生产级大模型推理内核，也不是某个模型 API 的简单封装。项目想探索的是：如何用 Rust 把推理外层的控制系统做得更明确、可测试、可本地运行、可审计。

链接：

- GitHub: https://github.com/yanghao1143/rust-norion
- Gitee: https://gitee.com/babalibaba/rust-norion
- Contributor Zone: https://github.com/yanghao1143/rust-norion/blob/main/docs/contributor-zone.md
- Reasoning Genome Chain: https://github.com/yanghao1143/rust-norion/blob/main/docs/architecture/reasoning-genome-chain.md

需要贡献者参与的方向：

- Rust 控制层架构、routing、reflection、scheduler、writer gate
- memory / KV / gist / retrieval hygiene / experience replay
- runtime adapter trait、manifest、command runtime、device profile
- benchmark、CI、trace schema、可复现验证
- 文档、runbook、架构图、贡献者 onboarding
- clean-room、许可证边界、隐私和治理

欢迎 issue、架构 review 和小而准的 PR。
