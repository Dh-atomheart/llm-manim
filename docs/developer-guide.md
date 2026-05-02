# 开发者手册

本文面向需要从源码运行、调试或修改 LLM-Manim 的开发者。

## 环境要求

请先准备：

- Node.js 和 npm
- Rust stable toolchain
- Tauri 2 所需系统依赖
- Python、uv、FFmpeg、MiKTeX/LaTeX，用于实际渲染验证

Windows 是当前主要开发和发布目标。macOS/Linux 可能需要额外验证。

## 安装依赖

在仓库根目录执行：

```powershell
npm install
```

## 本地运行

启动 Tauri 开发模式：

```powershell
npm run tauri dev
```

仅启动前端开发服务器：

```powershell
npm run dev
```

前端 dev server 由 Vite 提供，Tauri 配置中的 `devUrl` 为 `http://localhost:1420`。

## 项目结构

```text
src/                  React 前端、视图、组件、状态和 Tauri command client
src-tauri/            Tauri/Rust 后端、SQLite migrations、渲染与 Provider 服务
docs/                 用户、开发、发布和维护手册
tests/e2e/            Playwright 端到端测试
```

## 前后端边界

前端通过 `@tauri-apps/api/core` 的 `invoke` 调用 Rust command。前端 command client 位于 `src/commands/`，后端命令位于 `src-tauri/src/commands/`。

主要命令域：

- workspace：workspace 初始化、状态读取、runtime 检查。
- provider：Provider 增删改查和连接测试。
- project：项目管理。
- job：提交、取消、重试、删除任务，读取日志和渲染产物。
- settings：生成设置。

## 渲染流程

高层流程：

1. 用户提交 prompt。
2. 后端创建 `prompt_jobs` 记录并入队。
3. Provider 生成 Manim Python 代码。
4. 静态检查器验证代码安全和 ManimCE 兼容性。
5. Rust 后端写入 `generated_scene.py` 和 `manim.cfg`。
6. 后端通过 `uv run --with manim manim ...` 渲染 MP4。
7. 产物写入 workspace 的 `artifacts/` 并记录到 SQLite。

## 常用命令

```powershell
npm run build
npm test
npm run test:e2e
cargo check
npm run tauri -- build
```

`cargo check` 应在 `src-tauri/` 目录执行，或从根目录指定 manifest。若仓库目录被重命名后出现旧路径错误，先运行：

```powershell
cd src-tauri
cargo clean
cargo check
```

## 修改建议

- 优先保持 Tauri command 返回统一的 `AppResponse<T>`。
- 涉及数据库 schema 时新增 migration，不要直接修改已有 migration 的语义。
- 修改渲染或静态检查逻辑后，至少运行 `cargo check` 和相关测试。

