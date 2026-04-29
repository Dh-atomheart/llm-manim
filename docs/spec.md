# LLM 驱动 Manim 动画生成工具规格说明书（V1）

## 1. 文档信息
- 文档名称：LLM-Manim App 规格说明书
- 版本：v1.1
- 日期：2026-04-29
- 适用范围：V1（Windows 本地桌面版）
- V1 定位：面向教师/教研人员的本地 Manim 动画生成工具

## 2. 背景与目标
### 2.1 背景
本产品旨在降低 Manim 使用门槛。用户通过自然语言输入教学或科普动画需求，系统调用用户配置的大模型服务（兼容 OpenAI 协议或 Anthropic 协议）生成 Manim 动画脚本，并在本机托管的 Manim 环境中渲染为视频。

### 2.2 核心目标
V1 的首要目标是稳定打通“提示词 -> Manim 渲染 -> 可播放 MP4 -> 视频预览 -> 历史记录”的本地闭环。

### 2.3 业务目标
- 让不懂 Manim 编程的教师/教研人员也能生成基础教学动画。
- 优先保证生成链路稳定、失败可诊断、任务可恢复。
- 保持本地部署优先，降低云端运维、账号体系和协作复杂度。

### 2.4 V1 成功定义
- 运行时“成功”定义为任务产出可播放 MP4，并可在应用内预览。
- 动画画面质量通过验收样例和人工测试评估，不作为每次生成任务的自动通过条件。
- V1 不承诺运行时自动识别遮挡、错位、闪烁或教学逻辑错误。

## 3. 目标用户
### 3.1 首批用户
- 教师
- 教研人员
- 课程内容制作人员

### 3.2 次要用户
- 学生/自学者
- 科普视频创作者

V1 的 UI 文案、错误提示和验收样例优先服务教师/教研场景。

### 3.3 UI 设计风格
V1 采用简洁、线条化、工具型桌面应用风格。界面以黑白灰为主，彩色仅用于状态、警告、错误、成功和关键强调；不使用拟物、立体阴影、玻璃、渐变或装饰性视觉效果。详细规范见 [ui-design.md](ui-design.md)。

## 4. 范围定义
### 4.1 V1 包含
- 平台：Windows 本地桌面应用。
- 技术栈：Tauri + React + TypeScript + npm。
- 本地后端：由 Tauri/Rust 侧负责任务编排、Provider 调用、Manim 渲染进程管理和本地存储。
- 运行环境：应用托管 Python、Manim Community Edition、FFmpeg 等运行依赖，并提供环境检查与修复引导。
- 内容类型：
  - 数学公式与推导
  - 几何图形与变换
  - 物理示意动画
  - 算法/代码可视化
- 交互模式：单轮生成（纯提示词）。
- 输入方式：V1 仅提供空输入框，不提供内置提示模板或模板库。
- 代码可见性：用户不编辑、不查看、不复制生成的 Manim 源码。
- LLM 输出协议：V1 要求模型返回包含 Manim 代码的 Markdown 代码块；应用负责解析、校验和执行，模型不得直接执行本地命令。
- 输出格式：仅 MP4。
- 默认输出参数：720p / 30fps。
- 供应商接入：同时支持 OpenAI 兼容与 Anthropic 兼容；DeepSeek 通过 OpenAI 兼容配置接入，不新增独立 Provider 类型。
- 项目能力：项目列表、生成历史、简化日志、手动重试、手动取消。

### 4.2 V1 不包含
- 多轮对话生成。
- 自动重试策略。
- 渲染失败后自动调用模型修复脚本。
- 自动质量评分或自动视觉审查。
- 用户编辑、查看或导出 Manim 源码。
- 云端多人协作、账号体系、权限管理。
- 可视化拖拽时间轴编辑。
- GIF/WebM 导出。
- 内容合规拦截系统。
- 内置提示模板库。

## 5. 关键决策（已确认）
- 仅 Windows 平台。
- V1 首要目标是稳定生成闭环，而不是最高动画质量。
- 生成时长不设硬性上限，但需要软限制提醒、状态反馈和手动取消。
- 提示词长度不设硬性上限，但应用不得因长输入崩溃。
- 失败仅支持手动重试，不自动重试、不自动修复。
- 本机渲染并发默认 1（串行）。
- 首次使用不设默认模型，用户必须手动选择 Provider 与模型。
- 首次启动要求用户选择全局工作区目录。
- 历史项目默认永久保留，用户手动删除。
- API Key 本地明文存储，UI 必须明确提示该风险。
- 用户不需要查看或复制生成的 Manim 源码。
- 模型只负责生成内容或请求固定工具，所有本地副作用（写文件、执行 Manim、删除文件、读取日志）必须由 Tauri/Rust command 编排。
- 元数据存储采用 SQLite；工作区内保存数据库、配置、日志、产物、临时任务文件和半托管 uv runtime。
- Manim 运行时第一阶段采用工作区内半托管 uv 环境，不承诺 V1 完整离线打包。
- V1 自动验收只做硬质量检查：MP4 存在、非空、可读取时长、渲染日志无 fatal；不做抽帧视觉评分或教学质量自动评分。
- Provider HTTP 调用由 Rust 后端使用 `reqwest`。
- SQLite 由 Rust 后端使用 `sqlx` 管理。
- uv/Manim 子进程由 Rust 后端使用 `tokio::process` 管理。
- 前端状态管理使用 Zustand，样式使用 CSS Modules。

## 6. 用户流程
### 6.1 首次启动流程
1. 用户启动应用。
2. 应用要求用户选择全局工作区目录。
3. 应用检查托管 Manim 运行环境（Python、Manim、FFmpeg 等）。
4. 若依赖缺失或损坏，应用展示修复引导。
5. 用户新增 Provider 配置（base_url、api_key、model）。
6. 应用提示 API Key 将明文保存在本机配置中。
7. 用户执行连接测试。
8. 连接测试成功后，用户可新建项目。

### 6.2 主流程
1. 新建项目。
2. 输入单轮自然语言提示词。
3. 选择供应商与模型。
4. 点击生成。
5. 系统编排 Prompt，并注入 Manim 规则知识。
6. 系统调用 LLM 生成包含 Manim 代码的 Markdown 代码块。
7. 系统解析代码块并执行静态校验。
8. 任务进入串行渲染队列。
9. 本机半托管 uv/Manim 环境渲染 MP4。
10. 系统执行硬质量检查。
11. 产出可播放 MP4 后，应用内预览视频。
12. 生成记录写入项目历史。

### 6.3 异常流程
- API 鉴权失败：提示 Key 无效、模型无权限或配置错误。
- 网络异常：提示超时/不可达，并建议测试连接后手动重试。
- 依赖缺失：提示缺失组件，并给出修复引导。
- 渲染失败：展示可读错误摘要与下一次提示词改写建议，不展示源码。
- 用户取消：停止当前任务，将状态标记为 cancelled，并允许继续处理后续任务。
- 长任务：显示耗时提醒和取消入口，不强制终止。

## 7. 功能需求（FR）
### 7.1 项目与历史管理
- FR-001：支持项目新建、删除、列表查看。
- FR-002：每次生成记录必须可回看，至少包含状态、时间、Provider、模型、输出文件路径和简化日志。
- FR-003：历史默认永久保留，允许用户手动删除项目及其关联记录。
- FR-004：首次启动必须选择全局工作区目录；项目数据、视频产物、日志和元数据默认保存在该目录下。

### 7.2 提示词与生成
- FR-005：支持单次自然语言输入并触发生成。
- FR-006：V1 仅提供空输入框，不提供内置提示模板。
- FR-007：不限制提示词长度，但长输入不得导致应用崩溃；必要时可在任务日志中提示输入过长带来的模型或渲染风险。
- FR-008：生成流程必须可追踪状态（queued/running/succeeded/failed/cancelled）。
- FR-009：生成失败后不自动调用模型修复；用户可手动重试。
- FR-009A：LLM 输出必须包含一个可解析的 Python Markdown 代码块；解析失败时任务进入 failed，并返回 E_LLM_OUTPUT_INVALID。
- FR-009B：模型不得输出可执行 Shell 命令、绝对输出路径或本地文件操作指令；应用只提取代码块并走静态校验。

### 7.3 供应商与模型配置
- FR-010：支持新增、编辑、删除 OpenAI 兼容配置（base_url/api_key/model）。
- FR-011：支持新增、编辑、删除 Anthropic 兼容配置（base_url/api_key/model）。
- FR-012：支持“连接测试”并返回可读结果。
- FR-013：首次生成前必须显式选择 Provider 与模型。
- FR-014：保存 API Key 前必须提示“密钥将明文保存在本机配置中”。
- FR-014A：DeepSeek 作为 OpenAI 兼容配置接入，由用户填写或选择 base_url 与 model，不新增 deepseek 专用 provider_type。

### 7.4 渲染与队列
- FR-015：渲染任务默认串行执行（并发=1）。
- FR-016：支持手动取消当前任务。
- FR-017：任务失败后支持一键手动重试，复用原提示词、Provider 与模型。
- FR-018：输出视频格式为 MP4，默认 720p/30fps。
- FR-019：不设置硬性任务时长上限；长任务必须有耗时反馈和取消入口。
- FR-019A：渲染命令由应用固定模板生成，模型输出不得影响可执行命令、工作目录或输出目录。
- FR-019B：渲染完成后必须执行硬质量检查；检查失败时任务状态为 failed，错误码为 E_ARTIFACT_INVALID。

### 7.5 结果预览与日志
- FR-020：支持内置视频预览（播放、暂停、拖动进度）。
- FR-021：向普通用户展示简化日志面板。
- FR-022：失败场景需展示可读错误摘要与下一步建议。
- FR-023：失败时提供下一次提示词改写建议，但不自动重试、不自动修复。
- FR-024：内部日志可保留完整渲染错误用于排查；普通用户界面不展示生成源码。

### 7.6 Manim 能力约束
- FR-025：V1 仅使用 Manim Community Edition。
- FR-026：生成阶段需注入 Manim 规则知识（best practices/示例片段）以提高可执行率。
- FR-027：应用负责检查托管 Manim 环境是否可用，并在设置页或首次启动流程中展示检查结果。
- FR-028：V1 强制使用 Manim Community Edition，静态校验必须拒绝 ManimGL 入口（例如 `from manimlib import *`、`InteractiveScene`、`manimgl`）。
- FR-029：V1 必须拒绝包含危险本地副作用的生成代码，包括但不限于 `subprocess`、`os.system`、`socket`、`requests`、`eval`、`exec`、任意显式文件读写。

## 8. 非功能需求（NFR）
- NFR-001 稳定性：应用在失败任务、取消任务或渲染异常后可继续生成，不崩溃。
- NFR-002 可观测性：任务状态、关键阶段日志、失败原因和建议动作可见。
- NFR-003 可用性：核心路径操作清晰、步骤短，错误提示面向普通教师用户。
- NFR-003A UI 风格：界面必须遵循黑白灰主色、线条化组件、彩色语义提示的设计规范，不做立体化或装饰性视觉效果。
- NFR-004 渲染保护：V1 提供软限制提醒、手动取消、异常恢复和串行队列；CPU/内存/超时硬隔离作为后续增强，不作为 V1 必须项。
- NFR-005 数据本地化：项目数据、配置、日志和产物保存在用户选择的本机工作区目录中。
- NFR-006 质量要求：公式推导、几何变换、物理示意和算法可视化需通过验收样例评估；质量问题不作为每次运行时自动判定条件。
- NFR-007 安全提示：应用必须明确提示 API Key 明文存储风险。
- NFR-008 执行边界：模型输出永远不直接触发本地副作用；本地副作用只能通过白名单 Tauri command 执行。
- NFR-009 存储一致性：项目、任务、产物和日志元数据以 SQLite 为权威来源；文件系统只保存配置、产物、日志正文、临时脚本和 runtime 文件。
- NFR-010 迁移一致性：SQLite schema 变更必须通过受控迁移执行，迁移失败不得继续写入业务数据。

## 9. 系统架构（逻辑）
### 9.1 组件划分
- 桌面前端（React）：项目管理、提示词输入、Provider 配置、队列状态、日志、视频预览、工作区选择。
- Tauri 本地后端（Rust）：任务编排、Provider 调用、脚本生成结果处理、状态管理、本地文件与元数据读写。
- 渲染执行层：托管 Manim Community Edition 环境，负责将内部脚本渲染为 MP4。
- Provider 适配层：OpenAI 兼容与 Anthropic 兼容接口封装。
- 存储层：工作区目录中的元数据、日志、视频产物和明文 Provider 配置。

### 9.2 关键链路
用户提示词 -> Prompt 编排（含 ManimCE 规则注入） -> LLM 生成 Markdown 代码块 -> 代码块解析 -> 静态校验 -> 任务入队 -> 半托管 uv/Manim 渲染 -> 硬质量检查 -> MP4 产出 -> 应用内预览 -> 项目历史归档。

### 9.3 环境检查链路
应用启动或进入设置页 -> 检查工作区目录 -> 检查 `.runtime` 半托管 uv 环境 -> 检查 Python/Manim/FFmpeg/LaTeX -> 运行最小 Manim 自检 -> 展示可用/不可用状态 -> 不可用时展示修复引导。

### 9.4 详细规格索引
- 文档阅读顺序与权威范围：见 [index.md](index.md)。
- 架构职责边界：见 [architecture.md](architecture.md)。
- V1 技术栈锁定：见 [tech-stack.md](tech-stack.md)。
- 前端实现、状态管理、command client 与参考 UI 迁移：见 [frontend-implementation.md](frontend-implementation.md)。
- Tauri command 与错误格式：见 [api-contract.md](api-contract.md)。
- 工作区目录、SQLite 表与清理策略：见 [workspace-storage.md](workspace-storage.md)。
- SQLite schema 迁移策略：见 [database-migrations.md](database-migrations.md)。
- Provider 调用协议、DeepSeek 接入与错误映射：见 [provider-protocol.md](provider-protocol.md)。
- Prompt 拼装与 LLM 输出合同：见 [prompt-contract.md](prompt-contract.md)。
- LLM 调用编排、阶段日志和失败落库：见 [llm-orchestration.md](llm-orchestration.md)。
- Python AST 静态校验与危险能力拦截：见 [static-checker.md](static-checker.md)。
- 半托管 uv/Manim runtime 管理：见 [runtime-management.md](runtime-management.md)。
- LLM 代码解析、静态校验、渲染状态机与硬质量检查：见 [render-pipeline.md](render-pipeline.md)。
- 日志、脱敏、错误摘要与可观测性：见 [logging-observability.md](logging-observability.md)。
- 模型输出、command、路径、API Key 与运行隔离安全边界：见 [security-boundary.md](security-boundary.md)。
- UI 视觉风格、布局和状态表达：见 [ui-design.md](ui-design.md)。
- UI 页面结构与文本线框：见 [ui-wireframes.md](ui-wireframes.md)。
- 测试工具链、golden prompts 与失败场景：见 [test-plan.md](test-plan.md)。
- V1 开发里程碑：见 [milestones/index.md](milestones/index.md)。

## 10. 数据模型（最小）
### 10.1 WorkspaceConfig
- workspace_path
- created_at
- updated_at

V1 元数据以 SQLite 为权威存储，具体表结构见 [workspace-storage.md](workspace-storage.md)。

### 10.2 Project
- id
- name
- created_at
- updated_at

### 10.3 ProviderConfig
- id
- provider_type（openai_compatible / anthropic_compatible）
- base_url
- api_key（明文，V1 按当前要求）
- model
- created_at
- updated_at

### 10.4 PromptJob
- id
- project_id
- provider_id
- prompt_text
- state（queued/running/succeeded/failed/cancelled）
- error_code
- error_summary
- suggestion
- created_at
- started_at
- finished_at

### 10.5 RenderArtifact
- id
- job_id
- video_path
- resolution
- fps
- duration
- created_at

### 10.6 JobLog
- id
- job_id
- stage
- level
- message
- timestamp

## 11. 错误码与提示策略
### 11.1 错误码
- E_AUTH_401：鉴权失败，检查 API Key、模型权限或 Provider 配置。
- E_NET_TIMEOUT：网络超时，建议测试连接后手动重试。
- E_PROVIDER_ERROR：模型服务返回错误，检查 base_url、模型名称或服务状态。
- E_PROVIDER_RESPONSE_INVALID：模型服务响应结构无效、缺少文本内容或无法解析为预期格式。
- E_LLM_OUTPUT_INVALID：模型输出无法解析出唯一 Python Markdown 代码块，或代码块不满足基础格式。
- E_STATIC_CHECK_FAILED：生成代码未通过 ManimCE、危险 API 或 Scene 结构校验。
- E_RENDER_FAIL：渲染失败，查看摘要并尝试改写提示词。
- E_ARTIFACT_INVALID：渲染产物缺失、为空、不可读取或时长为 0。
- E_DEP_MISSING：依赖缺失或托管环境不可用，按修复引导处理。
- E_WORKSPACE_INVALID：工作区不可写、被删除或路径不可访问。
- E_CANCELLED：用户主动取消。

### 11.2 提示原则
- 用户可读优先。
- 明确“原因 + 影响 + 建议动作”。
- 普通用户界面不展示生成源码。
- 简化日志展示关键阶段；内部日志可记录完整 Provider 和渲染错误。
- 失败提示应给出下一次提示词改写建议，但不得自动发起二次模型调用。

## 12. 隐私与安全限制
- API Key 按 V1 决策明文保存在本机配置中。
- 应用保存密钥前必须展示明文存储风险提示。
- 用户提示词会发送到用户选择的模型服务商或兼容服务。
- 项目数据、日志和视频产物仅默认保存在用户选择的本机工作区目录。
- Provider 配置、项目元数据、任务元数据、产物元数据和日志索引保存在工作区 SQLite 数据库中。
- API Key 持久化时不得写入渲染日志、错误详情或前端调试日志。
- V1 不提供账号体系、云同步或远程协作。
- V1 不提供内容合规拦截，不对用户输入内容做政策审查。

详细安全边界见 [security-boundary.md](security-boundary.md)，日志脱敏规则见 [logging-observability.md](logging-observability.md)。

## 13. 验收标准
### 13.1 首次启动
1. 用户可选择全局工作区目录。
2. 应用可展示托管 Manim 环境检查结果。
3. 用户可新增 Provider 配置，并看到 API Key 明文存储提示。
4. 连接测试可返回成功或可读失败原因。

### 13.2 主链路
1. 用户输入中文公式推导提示词后，可生成并预览 MP4。
2. 生成记录包含状态、时间、Provider、模型、输出路径和简化日志。
3. LLM 输出 Markdown 代码块解析、静态校验、渲染、硬质量检查均有明确状态。
4. 任务失败后应用不崩溃，用户可手动重试。

### 13.3 四类样例
1. 公式推导：公式排版基本正确，步骤清晰，无明显遮挡。
2. 几何变换：移动/旋转/缩放过程连贯，无明显跳帧或错位。
3. 物理示意：关键对象位置关系稳定，画面表达与说明一致。
4. 算法可视化：步骤表达清楚，状态变化可理解。

### 13.4 失败与长任务
1. 鉴权失败、网络超时、依赖缺失、渲染失败、用户取消均有可读提示。
2. 长任务运行中可看到状态反馈和取消入口。
3. 用户取消当前任务后，队列可继续处理下一个任务。

## 14. 测试计划
V1 测试工具链、mock Provider、fake Manim、golden prompts、迁移、日志、安全边界、失败场景和发布前检查以 [test-plan.md](test-plan.md) 为准。

最小测试范围：

- 首次启动：选择工作区、环境检查、Provider 配置、连接测试。
- 主链路：中文提示词生成公式推导动画，成功产出可播放 MP4。
- 验收样例：公式推导、几何变换、物理示意、算法可视化。
- 失败场景：鉴权失败、网络超时、Provider 响应无效、依赖缺失、静态校验失败、Manim 渲染失败、用户取消。
- 历史记录：生成记录、状态、时间、视频路径、简化日志可回看；用户可删除项目。
- 长任务：任务运行中可取消，取消后应用不崩溃，队列可继续处理下一个任务。

## 15. 里程碑
V1 采用 M1-M6 里程碑推进，完整规划见 [milestones/index.md](milestones/index.md)。

阶段总览：

- M1：Project Foundation。
- M2：Workspace & Storage。
- M3：Provider Configuration。
- M4：Generation & Render Pipeline。
- M5：User-Facing Flow。
- M6：Acceptance & Hardening。

## 16. 风险与约束
### 16.1 高风险
- API Key 本地明文存储：存在泄露风险，必须在 UI 中明确提示。
- 托管 Manim 环境打包与修复复杂，可能影响安装包体积和首次启动体验。

### 16.2 中风险
- 不限制时长与提示词长度：可能引发资源占用过高或队列长时间阻塞。
- 串行执行：吞吐量低，长任务可能影响体验。
- 无自动重试和自动修复：网络抖动或一次性脚本错误会降低成功率。
- 不展示源码：降低高级用户自助排查能力，需要更好的错误摘要。

### 16.3 缓解建议
- 提供手动取消、耗时提醒和明确进度反馈。
- 提供环境检查与修复引导。
- 优化错误摘要和提示词改写建议。
- 后续版本引入可选本地加密存储、自动修复和更强运行隔离。

## 17. 未来版本建议（非 V1 承诺）
- 多轮对话生成。
- 供应商自动切换与成本优化。
- 自动重试与智能降级。
- 渲染失败后的自动脚本修复。
- 内容合规与版权策略模块。
- 多平台支持（macOS/Linux）。
- 工程源码导出与高级模板库。
- 运行时自动质量评分或视觉检查。
