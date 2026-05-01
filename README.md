# LLM-Manim V1

LLM-Manim V1 是一个面向 Manim Community Edition 的桌面应用。它使用 LLM 根据用户提示生成 Manim 动画代码，在本地进行静态检查、渲染 MP4，并保存生成历史与日志，帮助用户更快制作数学、物理、算法和课程讲解类动画。

项目仓库名为 `manim4learn`，应用公开名称为 `LLM-Manim V1`。

## 核心功能

- 通过 OpenAI-compatible 或 Anthropic-compatible Provider 生成 ManimCE 动画代码。
- 对 LLM 生成的 Python/Manim 代码进行静态检查，降低不安全或不兼容代码进入渲染流程的风险。
- 调用本地 ManimCE runtime 渲染 MP4，并在应用中预览渲染结果。
- 使用 SQLite 保存 workspace、项目、Provider 配置、生成任务、渲染产物和日志。
- 提供任务历史、状态追踪、错误摘要和基础排查信息。

## 技术栈

- 桌面框架：Tauri 2
- 前端：React 19、TypeScript、Vite、Zustand、CSS Modules
- 后端：Rust、Tauri commands、Tokio、reqwest、serde
- 存储：SQLite、sqlx migrations
- 动画 runtime：Python、uv、Manim Community Edition、FFmpeg、LaTeX/MiKTeX
- 测试：Vitest、Playwright

## 环境依赖

开发和本地打包前，请先准备以下工具：

- Node.js 和 npm
- Rust stable toolchain
- Tauri 2 所需系统依赖
- Python
- uv
- Manim Community Edition
- FFmpeg
- LaTeX 发行版，例如 MiKTeX

Windows 是 V1 的主要目标平台。macOS 和 Linux 可能需要额外适配与验证。

## 快速开始

克隆仓库后，在项目根目录执行：

```powershell
npm install
npm run tauri dev
```

首次启动后，按应用界面完成基础配置：

1. 选择或创建 workspace。
2. 配置 Provider，支持 OpenAI-compatible 或 Anthropic-compatible。
3. 填写 `base_url`、`api_key` 和 `model`。
4. 确认本机已安装并可调用 ManimCE、FFmpeg 和 LaTeX。
5. 输入提示词，生成并渲染 Manim 动画。

## 常用命令

```powershell
# 仅启动前端开发服务器
npm run dev

# 前端类型检查并构建
npm run build

# 运行单元测试
npm test

# 运行端到端测试
npm run test:e2e

# 构建 Tauri 桌面应用安装包
npm run tauri -- build
```

Tauri 开发模式通常使用：

```powershell
npm run tauri dev
```

## 项目结构

```text
src/                  React 前端、视图、组件、状态和 Tauri command client
src-tauri/            Tauri/Rust 后端、SQLite migrations、渲染与 Provider 服务
docs/                 架构、接口、测试、runtime、发布准备等项目文档
references/           ManimCE API manifest、规则、示例和参考资料
tests/e2e/            Playwright 端到端测试
```

## 配置与数据

- Workspace 由用户在应用中选择，用于保存项目数据、生成任务、日志和渲染产物。
- Provider 配置包含 `base_url`、`api_key`、`model` 等信息。
- 生成任务会经过 LLM 调用、代码提取、静态检查、Manim 渲染、产物校验和结果展示。
- 渲染产物以本地 MP4 文件为主，应用通过 Tauri asset protocol 进行预览。

## 安全说明

- API Key 仅用于本地后端向配置的 Provider 发起请求。
- LLM 输出的代码在进入渲染流程前会经过静态检查。
- 渲染依赖本地 Python、ManimCE、FFmpeg 和 LaTeX runtime。
- 不要提交本地 runtime、构建产物、日志、API Key 或 workspace 私有数据。

## 当前状态

LLM-Manim V1 当前是一个本地桌面应用，主要面向 Windows 开发、运行和打包。仓库已经包含前端、Tauri/Rust 后端、SQLite migrations、静态检查器、渲染管线和测试配置。

更多设计和实现细节见 [docs](docs/)。

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.
