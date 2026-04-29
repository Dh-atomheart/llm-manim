# LLM-Manim V1 架构规格

## 1. 目标
本文定义 V1 的工程职责边界。实现者应按本文拆分 React 前端、Tauri/Rust 后端、Provider 层、SQLite 存储、半托管 uv runtime 与 Manim 渲染执行层。

核心约束：

- 模型只生成内容或请求工具，不直接执行本地副作用。
- 所有本地副作用都由白名单 Tauri command 进入 Rust 后端。
- Rust 后端是状态、文件、进程、日志和渲染的唯一编排者。

## 2. 总体架构
```text
React UI
  -> Frontend Command Client
  -> Tauri invoke(command, payload)
  -> Rust Command Layer
  -> Domain Services
     -> Workspace Service
     -> Migration Service
     -> SQLite Repository
     -> Provider Service
     -> Prompt/Skill Service
     -> LLM Orchestrator Service
     -> Render Queue
     -> Runtime Service
     -> Logging/Observability Service
  -> File System / uv / Manim / FFmpeg / SQLite
```

关键技术栈以 [tech-stack.md](tech-stack.md) 为准。V1 前端使用 React、TypeScript、Zustand 和 CSS Modules；后端使用 Tauri v2、Rust、`sqlx`、`reqwest` 和 `tokio::process`。

## 3. 模块职责
### 3.1 React 前端
负责：

- 首次启动流程与工作区选择。
- 项目列表、项目详情、生成面板。
- Provider 配置表单、连接测试入口。
- 任务状态展示、取消、手动重试。
- 简化日志展示。
- MP4 预览。
- 展示 API Key 明文存储风险提示。
- 使用 Zustand 管理前端派生状态，后端/SQLite 仍是任务状态权威来源。
- 使用 CSS Modules 实现页面样式，避免引入重型 UI 组件库。
- 执行 `docs/ui-design.md` 定义的 UI 风格：黑白灰为主、线条化组件、彩色仅作语义提示，不使用立体阴影、玻璃、渐变或装饰性视觉效果。
- 前端实现细节见 [frontend-implementation.md](frontend-implementation.md)。

不得负责：

- 直接访问工作区文件。
- 直接调用 DeepSeek/OpenAI/Anthropic API。
- 直接执行 Manim、uv、FFmpeg 或 Shell 命令。
- 自行维护任务状态机。
- 直接读取或拼接本地视频文件路径。
- 引入与 V1 设计规范冲突的营销式、拟物式或重装饰界面。

### 3.2 Tauri/Rust Command Layer
负责：

- 暴露白名单 command。
- 统一返回 `{ ok, data, error }`。
- 将前端请求转换为 domain service 调用。
- 将内部错误映射为产品错误码。

不得负责：

- 在 command 内写复杂业务逻辑。
- 返回不一致的错误结构。
- 接受前端传入的可执行命令或任意文件路径作为执行依据。

### 3.3 Workspace Service
负责：

- 初始化和验证工作区目录。
- 创建标准目录结构。
- 打开和迁移 SQLite 数据库。
- 检查工作区可读写。
- 维护 `.runtime` 半托管 uv 环境状态。

### 3.4 SQLite Repository
负责：

- 项目、Provider、任务、产物、日志索引的持久化。
- 状态迁移的原子更新。
- 查询项目历史与任务日志。
- 使用 `sqlx` 访问 SQLite。

原则：

- SQLite 是元数据权威来源。
- 文件系统路径必须以 workspace 相对路径保存为主，必要时运行时解析为绝对路径。

### 3.4A Migration Service
负责：

- 打开 workspace 数据库后执行 schema migration。
- 维护 schema 版本。
- 在迁移失败时返回 `E_DB`。
- 阻止迁移失败后的业务写入。

迁移策略见 [database-migrations.md](database-migrations.md)。

### 3.5 Provider Service
负责：

- 管理 OpenAI-compatible 与 Anthropic-compatible 配置。
- 连接测试。
- 发送 LLM 请求。
- 收集 Provider 错误并映射为统一错误码。
- 使用 `reqwest` 作为唯一 HTTP 客户端。

V1 约束：

- DeepSeek 通过 `openai_compatible` 配置接入。
- 不新增 `deepseek` provider_type。
- Provider Service 不执行本地工具；tool call 只作为模型请求，实际执行由 Render/Validation service 决定。
- Provider Service 不解析 Markdown 代码块，不决定任务终态；整体 LLM 流程由 LLM Orchestrator Service 编排。

### 3.6 Prompt/Skill Service
负责：

- 组装系统提示词。
- 注入精选 ManimCE 规则。
- 明确要求模型输出 Python Markdown 代码块。
- 禁止模型输出 shell 命令、输出路径和本地执行指令。
- 固定 Prompt 拼装规则，具体见 [prompt-contract.md](prompt-contract.md)。

V1 默认规则来源：

- `references/skills/manim-composer`
- `references/skills/manimce-best-practices`

不得默认注入：

- `references/skills/manimgl-best-practices`

### 3.7 LLM Orchestrator Service
负责：

- 读取 PromptJob 与 ProviderConfig。
- 调用 Prompt/Skill Service 构建固定 Prompt。
- 调用 Provider Service 获取模型文本。
- 解析唯一 Markdown Python 代码块。
- 调用 Static Checker 校验代码。
- 写入阶段日志、错误码、错误摘要和建议动作。
- 将通过静态校验的代码与 SceneName 交给 Render Queue/Render Execution Layer。

V1 约束：

- 不使用 LangChain、LangGraph 或类似 agent 框架。
- 不执行 shell、Manim、uv 或 FFmpeg。
- 不写任意路径。
- 不做自动修复或多轮对话。
- 详细规格见 [llm-orchestration.md](llm-orchestration.md)。

### 3.8 Render Queue
负责：

- 串行执行渲染任务，并发固定为 1。
- 管理任务状态机。
- 支持取消 running 任务。
- 支持失败任务手动重试。
- 将 stdout/stderr 和阶段日志写入日志系统。
- 只消费已通过静态校验的代码和 SceneName，不直接接收模型原始输出。

### 3.9 Runtime Service
负责：

- 检查 `.runtime` 半托管 uv 环境。
- 检查 Python、Manim、FFmpeg、LaTeX/MiKTeX 可用性。
- 提供运行 Manim 的固定入口。
- 提供环境修复建议。
- 记录锁定版本和健康检查结果。

V1 不负责：

- 完整离线打包 Python/Manim/MiKTeX。
- 跨平台 runtime 分发。

### 3.10 Render Execution Layer
负责：

- 在受控 job 临时目录写入生成脚本。
- 使用固定命令模板调用 ManimCE。
- 使用 `tokio::process` 启动、读取和取消 uv/Manim 子进程。
- 写入渲染日志。
- 输出 MP4 到标准 artifacts 目录。
- 执行硬质量检查。

不得负责：

- 接受模型指定 shell 命令。
- 接受模型指定绝对输出路径。
- 运行 ManimGL。

### 3.11 Video Access Layer
负责：

- 为应用内 `<video>` 预览生成 Tauri asset URL。
- 为“打开文件”或“定位文件”提供受控 opener 入口。

不得负责：

- 向前端暴露任意本地绝对路径作为可操作路径。
- 允许前端打开未经 artifact 校验的文件。

### 3.12 Logging/Observability Service
负责：

- 统一阶段日志、内部日志和用户可读日志。
- 在写入前执行脱敏。
- 生成错误摘要和建议动作。
- 维护 JobLog 与文件日志之间的关系。

日志规格见 [logging-observability.md](logging-observability.md)。

## 4. 数据流
### 4.1 首次启动
```text
React 选择工作区
-> initialize_workspace
-> Rust 创建目录结构
-> Rust 初始化 SQLite
-> Runtime Service 检查 .runtime
-> React 展示环境状态
```

### 4.2 生成任务
```text
React submit_prompt_job
-> Rust 创建 PromptJob(queued)
-> Render Queue 取任务
-> LLM Orchestrator 构建 Prompt
-> Provider Service 调用模型
-> LLM Orchestrator 解析 Markdown 代码块
-> LLM Orchestrator 调用静态校验
-> 写入 job 临时脚本
-> Manim 渲染
-> 硬质量检查
-> RenderArtifact 入库
-> PromptJob(succeeded)
```

### 4.3 失败任务
```text
任意阶段失败
-> 写 JobLog
-> 写 error_code/error_summary/suggestion
-> PromptJob(failed)
-> React 展示可读错误与手动重试入口
```

## 5. 状态所有权
- 前端只展示状态，不自行推导最终状态。
- Rust 后端是任务状态唯一写入者。
- SQLite 是任务状态权威来源。
- 文件系统产物必须能由 SQLite 元数据定位。

## 6. 安全边界
详细安全边界见 [security-boundary.md](security-boundary.md)。

- 所有 command 必须校验输入。
- API Key 不得进入普通日志、渲染日志或错误 details。
- 模型输出必须通过 Markdown 代码块解析和静态校验。
- 静态校验采用 Python AST 与 denylist 组合，具体见 [static-checker.md](static-checker.md)。
- 静态校验失败不得写入可执行渲染脚本。
- 渲染脚本只能写入 job 临时目录。
- 渲染命令由应用固定模板生成。

## 7. 第一阶段实现顺序
1. Workspace Service + SQLite 初始化。
2. Tauri command 统一返回格式。
3. Provider 配置 CRUD 与连接测试。
4. PromptJob 状态机与串行队列。
5. LLM Orchestrator 编排 Provider、Prompt、解析与静态校验。
6. 半托管 uv/Manim 环境检查。
7. Manim 渲染与硬质量检查。
8. 前端页面接入。
