# M3 Provider Configuration

## 目标
实现 Provider 配置、API Key 明文存储风险提示、连接测试和日志脱敏。M3 完成后，用户可以配置 OpenAI-compatible 或 Anthropic-compatible 服务，并验证模型连接。

## 范围
包含：

- Provider 配置新增、编辑、删除、列表。
- API Key 明文存储确认提示。
- Provider 连接测试。
- DeepSeek 通过 `openai_compatible` 接入。
- Provider 相关日志脱敏。
- Provider 设置 UI。
- Provider 设置 UI 使用简洁表单和线条组件；API Key 风险使用警告色强调，但不使用大面积彩色背景。

不包含：

- 提示词生成任务。
- LLM Markdown 代码块解析。
- Manim 渲染。
- 自动获取模型列表。

## 前置依赖
- M2 Workspace & Storage
- `docs/api-contract.md`
- `docs/workspace-storage.md`
- `docs/provider-protocol.md`
- `docs/logging-observability.md`
- `docs/security-boundary.md`
- `docs/tech-stack.md`
- `docs/ui-design.md`
- `docs/ui-wireframes.md`
- `references/b_pos8dDmvcka`

## 主要任务
- 创建 `provider_configs` 表。
- 实现 `list_provider_configs`。
- 实现 `save_provider_config`。
- 实现 `delete_provider_config`。
- 实现 `test_provider_config`。
- Provider 列表接口不得返回 API Key。
- 保存 API Key 前，UI 必须展示明文存储风险提示。
- 风险提示必须符合 `docs/ui-design.md`：警告色配合明确文本，不只依赖颜色。
- 连接测试支持 OpenAI-compatible 与 Anthropic-compatible 的最小请求。
- DeepSeek 示例配置使用 `provider_type = openai_compatible`。
- Provider HTTP 调用由 Rust 后端使用 `reqwest` 完成，前端不得直连 Provider。
- 日志脱敏器覆盖 API Key、Authorization header、secret 字段。
- Provider 测试日志和错误摘要必须符合 `docs/logging-observability.md`。
- API Key 明文保存、列表不返回 Key、前端不持久化 Key 必须符合 `docs/security-boundary.md`。
- Provider 设置 UI 参考 `references/b_pos8dDmvcka/components/views/provider-settings.tsx`。
- API Key 风险提示、连接测试、Provider 列表、保存和删除操作应迁移参考 UI 的结构，但数据必须来自 Tauri command。
- 不保留引用设计中的 mock Provider、mock 测试结果或硬编码模型列表。

## 接口与数据影响
Tauri command：

- `list_provider_configs`
- `save_provider_config`
- `delete_provider_config`
- `test_provider_config`

SQLite 表：

- `provider_configs`
- `job_logs` 可在本阶段先建立，用于记录 provider 测试日志。

错误码：

- `E_AUTH_401`
- `E_NET_TIMEOUT`
- `E_PROVIDER_ERROR`
- `E_PROVIDER_RESPONSE_INVALID`
- `E_PROVIDER_IN_USE`
- `E_VALIDATION`
- `E_DB`

## 验收标准
- 用户可新增 OpenAI-compatible Provider。
- 用户可新增 Anthropic-compatible Provider。
- 用户保存 API Key 前能看到明文存储风险提示。
- Provider 表单无立体阴影、玻璃、渐变或装饰性视觉元素。
- Provider 列表不返回 API Key。
- 连接测试成功时返回可读成功结果。
- API Key 错误时返回 `E_AUTH_401` 或 Provider 可读错误。
- 网络超时时返回 `E_NET_TIMEOUT`。
- 日志和错误详情不泄露 API Key。
- 使用假 API Key 的脱敏测试不应在 UI、日志或错误 details 中泄露明文。
- DeepSeek 可作为 OpenAI-compatible 配置被保存和测试。
- Provider 设置页与 `references/b_pos8dDmvcka/components/views/provider-settings.tsx` 的布局和交互保持一致。
- Provider 设置页不得做成独立复杂向导、营销式配置页或大面积彩色警告页。

## 风险与处理
- 不同兼容 API 行为差异：连接测试只做最小调用，不承诺完整模型能力。
- 明文 Key 风险：保存前提示，日志脱敏，不在列表接口返回 Key。
- 删除 Provider 破坏历史任务：queued/running 引用时拒绝删除；历史任务保留 provider_id。
