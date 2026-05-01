# LLM-Manim V1 文档索引

## 1. 阅读顺序

开发者应按以下顺序阅读文档：

1. [spec.md](spec.md)：产品边界、V1 成功定义、功能与非功能需求。
2. [architecture.md](architecture.md)：前端、Tauri/Rust 后端、Provider、存储、runtime 和渲染执行层的职责边界。
3. [initialization.md](initialization.md)：Windows 开发环境准备和项目初始化手册。
4. [tech-stack.md](tech-stack.md)：V1 锁定的关键技术栈和禁止选型。
5. [frontend-implementation.md](frontend-implementation.md)：前端目录、Zustand、command client、CSS Modules 与参考 UI 迁移。
6. [api-contract.md](api-contract.md)：React 与 Tauri/Rust 之间的 command 合同、错误结构和状态迁移。
7. [workspace-storage.md](workspace-storage.md)：工作区目录、SQLite 表、日志脱敏、删除和清理策略。
8. [database-migrations.md](database-migrations.md)：SQLite schema 版本、迁移执行、失败回滚和开发期重建策略。
9. [provider-protocol.md](provider-protocol.md)：Provider 调用协议、DeepSeek 接入、连接测试、错误映射。
10. [prompt-contract.md](prompt-contract.md)：Prompt 拼装、ManimCE 规则注入和 LLM Markdown 输出协议。
11. [prompt-optimization.md](prompt-optimization.md)：提示词分层、Manim skill 精选注入、规则预算和质量回归策略。
12. [llm-orchestration.md](llm-orchestration.md)：LLM 编排、阶段日志、失败落库和 LangChain/LangGraph 取舍。
13. [static-checker.md](static-checker.md)：代码块解析后的 Python AST 校验和危险能力拦截。
14. [runtime-management.md](runtime-management.md)：半托管 uv/Manim runtime、版本锁定、检查、修复和进程管理。
15. [render-pipeline.md](render-pipeline.md)：从提示词到 MP4 的流水线、状态机、取消、重试和硬质量检查。
16. [logging-observability.md](logging-observability.md)：日志级别、阶段、脱敏、用户日志和错误摘要。
17. [security-boundary.md](security-boundary.md)：模型输出、command、路径、API Key 和运行隔离边界。
18. [ui-design.md](ui-design.md) 与 [ui-wireframes.md](ui-wireframes.md)：视觉风格和页面结构。
19. [test-plan.md](test-plan.md)：测试工具链、golden prompts、失败场景和验收方式。
20. [golden-prompts-manual-acceptance.md](golden-prompts-manual-acceptance.md)：四类 golden prompts 的真实人工验收记录模板。
21. [milestones/index.md](milestones/index.md)：M1-M6 开发里程碑。

## 2. 权威范围

- 产品范围以 `spec.md` 为准。
- 技术选型以 `tech-stack.md` 为准。
- 前端实现策略以 `frontend-implementation.md` 为准。
- 前后端接口以 `api-contract.md` 为准。
- 工作区和数据库以 `workspace-storage.md` 为准。
- SQLite 迁移策略以 `database-migrations.md` 为准。
- Provider 请求和错误映射以 `provider-protocol.md` 为准。
- Prompt 和 LLM 输出格式以 `prompt-contract.md` 为准。
- 提示词优化、Manim skill 注入策略和 prompt 质量回归以 `prompt-optimization.md` 为准。
- LLM 调用编排、阶段日志和失败落库以 `llm-orchestration.md` 为准。
- 静态校验以 `static-checker.md` 为准。
- runtime 和子进程管理以 `runtime-management.md` 为准。
- 渲染状态机和产物检查以 `render-pipeline.md` 为准。
- 日志、脱敏和错误摘要以 `logging-observability.md` 为准。
- 安全边界以 `security-boundary.md` 为准。
- UI 风格以 `ui-design.md` 为准，页面结构以 `ui-wireframes.md` 为准。
- 阶段拆分以 `milestones/index.md` 为准。

## 3. 当前开发可用性

- M1/M2：现有文档已足以开始项目初始化、工作区和 SQLite 基础能力开发。
- M3：必须同时参考 `provider-protocol.md` 和 `workspace-storage.md`。
- M4：必须同时参考 `llm-orchestration.md`、`prompt-contract.md`、`static-checker.md`、`runtime-management.md`、`logging-observability.md`、`security-boundary.md` 和 `render-pipeline.md`。
- M5：必须同时参考 `frontend-implementation.md`、`api-contract.md`、`ui-design.md` 和 `ui-wireframes.md`。
- M6：必须以 `test-plan.md` 作为验收入口，并使用 `golden-prompts-manual-acceptance.md` 记录真实人工验收结果。

## 4. 冲突处理

当文档之间出现冲突时：

1. 先按本文第 2 节的权威范围判断。
2. 若仍无法判断，优先选择更严格的安全边界。
3. 若影响实现，应先更新文档，再更新代码。
