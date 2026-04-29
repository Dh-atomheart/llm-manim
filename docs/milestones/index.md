# LLM-Manim V1 里程碑总览

## 目标
本文定义 V1 从项目初始化到验收硬化的开发里程碑。V1 总完成标准：

- Windows 本地 Tauri 应用可启动。
- 用户可选择工作区、配置 Provider、新建项目。
- 用户输入提示词后，应用可调用 OpenAI-compatible/Anthropic-compatible Provider，解析 LLM Markdown 代码块，渲染 ManimCE MP4。
- 生成记录、日志、产物可回看。
- 失败可诊断、可手动重试，运行中任务可取消。
- API Key 明文存储风险有明确提示，日志不得泄露 API Key。

## 里程碑顺序
1. [M1 Project Foundation](m1-project-foundation.md)
2. [M2 Workspace & Storage](m2-workspace-storage.md)
3. [M3 Provider Configuration](m3-provider-config.md)
4. [M4 Generation & Render Pipeline](m4-generation-render-pipeline.md)
5. [M5 User-Facing Flow](m5-user-facing-flow.md)
6. [M6 Acceptance & Hardening](m6-acceptance-hardening.md)

## 依赖关系
```text
M1 -> M2 -> M3 -> M4 -> M5 -> M6
```

说明：

- M1 提供项目骨架和基本 Tauri 通信能力。
- M2 提供工作区、SQLite、项目管理和最小 UI。
- M3 提供 Provider 配置、连接测试和日志脱敏。
- M4 提供真实生成与渲染后端闭环。
- M5 提供用户可操作的完整前端闭环。
- M6 提供验收样例、失败场景和交付前硬化。

## 全局 UI 设计基准
`references/b_pos8dDmvcka` 是 V1 UI 的验收参考。所有涉及 UI 的里程碑都必须复用该参考的页面结构、视觉层级和核心交互意图。

复用边界：

- 复用顶部栏、左侧项目栏、工作台、历史、Provider 设置、基础设置、日志展开、状态徽标和首次启动体验。
- 不直接采用引用项目的 Next.js runtime、Tailwind 默认样式体系或整包 shadcn/Radix 组件库。
- 引用项目中的 mock 数据、演示按钮、模拟失败逻辑、演示日志和硬编码路径不得作为产品行为保留。
- UI 实现仍必须遵循 `docs/tech-stack.md`、`docs/ui-design.md` 和 `docs/ui-wireframes.md`。

## 文档依据矩阵
| 里程碑 | 必须参考的规格 |
| --- | --- |
| M1 | `docs/initialization.md`、`docs/tech-stack.md`、`docs/frontend-implementation.md`、`docs/architecture.md`、`docs/api-contract.md`、`docs/ui-design.md`、`docs/ui-wireframes.md`、`references/b_pos8dDmvcka` |
| M2 | `docs/workspace-storage.md`、`docs/database-migrations.md`、`docs/frontend-implementation.md`、`docs/api-contract.md`、`docs/tech-stack.md`、`docs/ui-design.md`、`docs/ui-wireframes.md`、`references/b_pos8dDmvcka` |
| M3 | `docs/provider-protocol.md`、`docs/logging-observability.md`、`docs/security-boundary.md`、`docs/workspace-storage.md`、`docs/api-contract.md`、`docs/tech-stack.md`、`docs/ui-design.md`、`docs/ui-wireframes.md`、`references/b_pos8dDmvcka` |
| M4 | `docs/render-pipeline.md`、`docs/llm-orchestration.md`、`docs/logging-observability.md`、`docs/security-boundary.md`、`docs/database-migrations.md`、`docs/provider-protocol.md`、`docs/prompt-contract.md`、`docs/static-checker.md`、`docs/runtime-management.md`、`docs/api-contract.md`、`docs/workspace-storage.md`、`docs/ui-wireframes.md` |
| M5 | `docs/frontend-implementation.md`、`docs/logging-observability.md`、`docs/api-contract.md`、`docs/tech-stack.md`、`docs/ui-design.md`、`docs/ui-wireframes.md`、`references/b_pos8dDmvcka` |
| M6 | `docs/test-plan.md`、`docs/spec.md`、`docs/frontend-implementation.md`、`docs/database-migrations.md`、`docs/logging-observability.md`、`docs/security-boundary.md`、全部硬规格文档、`references/b_pos8dDmvcka` |

## 通用完成定义
每个里程碑完成时必须满足：

- 涉及的 Tauri command 与 `docs/api-contract.md` 一致。
- 涉及的文件与数据库行为与 `docs/workspace-storage.md` 一致。
- 涉及的 SQLite schema 变更必须与 `docs/database-migrations.md` 一致。
- 涉及的渲染行为与 `docs/render-pipeline.md` 一致。
- 涉及的技术选型与 `docs/tech-stack.md` 一致。
- 涉及的前端实现必须与 `docs/frontend-implementation.md` 一致。
- 涉及的 Provider 调用与 `docs/provider-protocol.md` 一致。
- 涉及的 Prompt 和静态校验分别与 `docs/prompt-contract.md`、`docs/static-checker.md` 一致。
- 涉及的 LLM 调用编排、模型输出解析和阶段失败落库必须与 `docs/llm-orchestration.md` 一致。
- 涉及的 runtime 行为与 `docs/runtime-management.md` 一致。
- 涉及的日志、脱敏和错误摘要必须与 `docs/logging-observability.md` 一致。
- 涉及的安全边界必须与 `docs/security-boundary.md` 一致。
- 涉及的页面结构与 `docs/ui-wireframes.md` 一致。
- 涉及的 UI 交付必须与 `references/b_pos8dDmvcka` 的页面结构和交互意图一致。
- 验收和回归范围与 `docs/test-plan.md` 一致。
- 不引入 `docs/spec.md` 明确排除的 V2 能力。
- 有可执行验收步骤，而不是只完成代码提交。

## V1 不在里程碑中承诺
- 自动修复失败脚本。
- 多轮对话生成。
- 抽帧视觉评分。
- 教学质量自动评分。
- 完整离线打包 Python/Manim/MiKTeX。
- 用户编辑或查看 Manim 源码。
