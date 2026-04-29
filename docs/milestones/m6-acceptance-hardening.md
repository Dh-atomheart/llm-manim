# M6 Acceptance & Hardening

## 目标
完成 V1 交付前硬化：四类 golden prompts、失败场景、长任务取消、日志脱敏、工作区清理、文档一致性和回归测试。M6 不新增大功能，只修复稳定性、可诊断性和验收问题。

## 范围
包含：

- 四类 golden prompts 验收。
- Provider 失败场景。
- LLM 输出异常场景。
- Manim 静态校验失败场景。
- 渲染失败和产物无效场景。
- 长任务取消。
- 日志脱敏验证。
- 工作区临时文件清理。
- 文档与实现一致性检查。
- UI 风格一致性检查。

不包含：

- 自动修复失败脚本。
- 抽帧视觉评分。
- 教学质量自动评分。
- 模板库。
- 发布安装包。
- 完整离线 runtime 打包。

## 前置依赖
- M5 User-Facing Flow
- `docs/spec.md`
- `docs/render-pipeline.md`
- `docs/workspace-storage.md`
- `docs/api-contract.md`
- `docs/test-plan.md`
- `docs/provider-protocol.md`
- `docs/static-checker.md`
- `docs/runtime-management.md`
- `docs/tech-stack.md`
- `docs/frontend-implementation.md`
- `docs/database-migrations.md`
- `docs/logging-observability.md`
- `docs/security-boundary.md`
- `docs/ui-design.md`
- `docs/ui-wireframes.md`
- `references/b_pos8dDmvcka`

## 主要任务
- 建立 golden prompts：
  - 公式推导：二次方程求根公式。
  - 几何变换：三角形旋转、平移和相似变换。
  - 物理示意：匀速圆周运动速度与向心加速度。
  - 算法可视化：二分查找或冒泡排序。
- 对每个 golden prompt 执行完整生成流程。
- 验证成功产物符合硬质量检查。
- 验证失败场景：
  - Provider 鉴权失败。
  - 网络超时。
  - 无 Markdown 代码块。
  - 多 Markdown 代码块。
  - ManimGL 代码。
  - 危险 API。
  - Manim 渲染失败。
  - MP4 缺失或 duration 为 0。
- 验证 running 任务取消。
- 验证 failed/cancelled 任务手动重试。
- 验证 API Key 不出现在 UI、日志、错误详情中。
- 验证数据库迁移顺序、重复执行、失败回滚和 `E_DB` 映射。
- 验证前端 command client、Zustand store 和 CSS Modules 组织不偏离 `docs/frontend-implementation.md`。
- 验证日志级别、阶段、脱敏和错误摘要符合 `docs/logging-observability.md`。
- 验证模型输出、路径、command、artifact 打开和 API Key 风险符合 `docs/security-boundary.md`。
- 验证 `temp/` 清理规则。
- 检查实现是否偏离 API 合同和存储规格。
- 检查 UI 是否符合 `docs/ui-design.md`：黑白灰主色、线条化组件、彩色只作语义提示、无立体装饰。
- 逐页对照 `references/b_pos8dDmvcka` 做 UI 一致性验收，不只检查抽象风格。
- 检查顶部栏、左侧项目栏、工作台、历史、Provider 设置、基础设置、日志展开、状态徽标和视频预览是否与参考 UI 保持一致。
- 检查不存在 Next.js runtime、Tailwind 默认样式体系、整包 shadcn/Radix、mock 演示逻辑、硬编码路径、源码展示或 API Key 展示。
- 按 `docs/test-plan.md` 执行 Rust、Vitest、Playwright、mock Provider 和 fake Manim 场景。
- 将 UI 一致性检查纳入 Playwright 验收：核心页面可访问、状态文案可见、主要操作入口位置与参考 UI 一致。

## 接口与数据影响
覆盖所有 V1 command：

- Workspace commands
- Provider commands
- Project commands
- Job commands
- Log and artifact commands

覆盖所有核心表：

- `workspace_config`
- `projects`
- `provider_configs`
- `prompt_jobs`
- `render_artifacts`
- `job_logs`

覆盖核心错误码：

- `E_AUTH_401`
- `E_NET_TIMEOUT`
- `E_PROVIDER_ERROR`
- `E_PROVIDER_RESPONSE_INVALID`
- `E_LLM_OUTPUT_INVALID`
- `E_STATIC_CHECK_FAILED`
- `E_RENDER_FAIL`
- `E_ARTIFACT_INVALID`
- `E_DEP_MISSING`
- `E_CANCELLED`

## 验收标准
- 四类 golden prompts 至少能完成硬质量成功标准，或失败时错误可诊断且可重试。
- 所有失败场景返回预期错误码。
- 取消 running 渲染不会导致应用崩溃或队列卡死。
- 手动重试创建新 job，不覆盖原 job。
- Provider 列表、日志、错误详情不泄露 API Key。
- 数据库迁移测试覆盖空库、重复启动、失败回滚。
- 前端未绕过 command client 直接访问 Provider、文件或 shell。
- 安全边界测试覆盖危险 API、ManimGL、路径越界和 artifact 打开校验。
- 工作区清理不会删除有效 artifact 和 running job 文件。
- `docs/spec.md`、`docs/api-contract.md`、`docs/workspace-storage.md`、`docs/render-pipeline.md` 与实现行为一致。
- 主要页面无拟物、玻璃、渐变、大面积彩色背景或立体阴影；状态表达不只依赖颜色。
- 主要页面与 `references/b_pos8dDmvcka` 对照无明显结构偏离。
- 前端实现未引入 Next.js runtime、Tailwind 默认样式体系或整包 shadcn/Radix。
- 引用设计中的 mock 数据、演示按钮、模拟失败逻辑、演示日志和硬编码路径均已移除或替换为真实 Tauri command 数据流。

## 风险与处理
- Golden prompts 受模型波动影响：V1 只要求硬质量和可诊断，不要求每次教学质量完美。
- 失败场景难以稳定模拟：为 Provider、LLM 输出解析、静态校验和 artifact 检查建立可控测试替身。
- 长任务取消在 Windows 上不稳定：必须单独做取消压力测试。
- 日志脱敏遗漏：使用包含假 API Key 的测试数据验证所有日志路径。
