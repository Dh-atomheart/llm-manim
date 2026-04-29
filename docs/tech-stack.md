# LLM-Manim V1 技术栈规格

## 1. 目标
本文锁定 V1 的关键技术选型，减少实现阶段的自由发挥。除非后续文档明确修订，M1-M6 均按本文执行。

## 2. 前端
锁定：

- React
- TypeScript
- npm
- Zustand
- CSS Modules
- Tauri JavaScript API

原则：

- 前端只通过 Tauri command 读写状态。
- 前端不直接调用 Provider API。
- 前端不直接访问工作区文件。
- 前端不执行 shell、Manim、uv 或 FFmpeg。
- 前端不拼接本地绝对路径。
- UI 不引入重型组件库；优先使用原生语义元素、轻量组件和 CSS Modules。

### 2.1 UI 设计迁移规则
`references/b_pos8dDmvcka` 是 UI 设计基准，但不是 V1 技术栈基准。迁移规则：

- 不引入 Next.js runtime；V1 仍是 Tauri + React 桌面应用。
- 不把 Tailwind 作为默认样式体系；引用设计中的 Tailwind class 应翻译为 CSS Modules。
- 不整包搬运未使用的 shadcn/Radix 组件库。
- 不保留引用项目中的 mock 数据、演示按钮、模拟失败逻辑或硬编码路径。
- 可按需复刻引用中的轻量组件外观和行为，例如 Button、Input、Textarea、Select、Dialog、StatusBadge、LogPanel、VideoPreview。
- 可保留 `lucide-react` 作为图标库候选，前提是项目初始化时明确记录依赖并按需引入图标。

## 3. 后端
锁定：

- Tauri v2
- Rust
- `sqlx` + SQLite
- `reqwest`
- `tokio::process`
- `serde`
- 统一应用错误类型

职责：

- Tauri command 是前端进入后端的唯一入口。
- SQLite 是项目、Provider、任务、产物和日志元数据的权威来源。
- `reqwest` 是 Provider 调用的唯一 HTTP 客户端。
- `tokio::process` 是 uv/Manim 子进程的唯一执行入口。
- 所有文件写入、进程执行、日志写入和状态迁移均由 Rust 后端负责。

## 4. 视频访问
锁定：

- 应用内预览使用 Tauri asset URL。
- 打开或定位文件使用 Tauri opener 能力。
- 前端展示相对路径或文件名，不展示工作区绝对路径。

规则：

- `get_render_artifact` 返回元数据和 workspace 相对路径。
- `get_video_file_url` 返回可供 `<video>` 使用的 asset URL。
- 若需要“打开文件位置”，必须通过后端校验 artifact 后调用 opener，不允许前端直接打开任意路径。

## 5. 明确不采用
- 不使用前端 HTTP 客户端直连 DeepSeek/OpenAI/Anthropic。
- 不使用 Next.js 作为 V1 应用框架。
- 不使用 Tauri Shell 插件作为前端可控命令入口。
- 不使用 ManimGL。
- 不使用 JSON Schema 作为 V1 LLM 输出协议。
- 不使用完整离线 runtime 打包作为 V1 必选项。
- 不引入大型 UI 组件库、整包 shadcn/Radix 组件库、3D 视觉风格、玻璃拟态、渐变背景或装饰性卡片系统。

## 6. 版本记录
V1 初始化时应在实际项目中记录以下版本：

- Node.js
- npm
- Rust
- Tauri
- React
- `sqlx`
- `reqwest`
- Python
- uv
- Manim Community Edition
- FFmpeg
- MiKTeX/LaTeX

运行时版本锁定策略见 [runtime-management.md](runtime-management.md)。
