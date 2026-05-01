# LLM-Manim V1 项目初始化手册（Windows 开发者版）

## 1. 文档目标
本文用于指导开发者在 Windows 上初始化 LLM-Manim V1 项目。V1 技术路线为：

- 桌面应用：Tauri v2
- 前端：React + TypeScript
- 前端包管理器：npm
- 本地后端：Tauri/Rust
- Manim 开发环境：uv 管理的 Python 环境
- 项目位置：在仓库根目录初始化应用代码

本文只覆盖“开发环境准备 + 项目脚手架初始化 + 基础验证”。最终产品中的托管 Python/Manim 运行时打包策略后续单独设计。

## 2. 当前仓库状态
当前仓库已有：

```text
docs/
  spec.md
.gitignore
```

后续初始化 Tauri 项目时，目标是在仓库根目录生成：

```text
package.json
src/
src-tauri/
index.html
vite.config.ts
tsconfig.json
```

由于根目录不是空目录，执行脚手架命令前应先确认不会覆盖现有文件。若脚手架工具提示目录非空，选择在当前目录继续初始化，并保留已有 `docs/`。

## 3. 前置工具
### 3.1 必需工具
在 Windows 开发机上需要准备：

- Windows 10/11
- PowerShell
- Git
- Node.js LTS
- npm
- Rust MSVC 工具链
- Microsoft C++ Build Tools
- Microsoft Edge WebView2 Runtime
- uv
- MiKTeX

Tauri Windows 开发依赖 Microsoft C++ Build Tools、WebView2、Rust 和 Node.js。Manim 开发期环境使用 `uv`，公式渲染需要 LaTeX 发行版，Windows 上优先使用 MiKTeX。

### 3.2 推荐安装命令
优先使用 `winget` 安装。若本机 `winget` 不可用，请改用对应官网安装包。

```powershell
winget install --id Git.Git -e
winget install --id OpenJS.NodeJS.LTS -e
winget install --id Rustlang.Rustup -e
winget install --id Microsoft.VisualStudio.2022.BuildTools -e
winget install --id Microsoft.EdgeWebView2Runtime -e
winget install --id MiKTeX.MiKTeX -e
```

uv 可使用官方安装脚本或包管理器安装：

```powershell
powershell -ExecutionPolicy ByPass -c "irm https://astral.sh/uv/install.ps1 | iex"
```

安装 Visual Studio Build Tools 时，需确保包含 C++ 桌面开发相关组件。安装后重新打开 PowerShell，让 PATH 生效。

## 4. 本机状态检查
在仓库根目录运行：

```powershell
node -v
npm.cmd -v
rustc --version
cargo --version
uv --version
git status --short
```

本机已探测到的状态：

```text
node v22.12.0
npm 11.0.0
rustc 1.94.1
cargo 1.94.1
uv 0.10.12
```

注意：当前 PowerShell 执行 `npm -v` 会被执行策略拦截，因为它会命中 `npm.ps1`。本项目文档统一使用 `npm.cmd`。

## 5. 初始化 Tauri + React + TypeScript 项目
### 5.1 在仓库根目录创建脚手架
进入仓库根目录：

```powershell
cd E:\manim4learn
```

执行 Tauri 初始化：

```powershell
npm.cmd create tauri-app@latest . -- --template react-ts
```

若工具进入交互模式，选择：

- Package manager：npm
- UI template：React
- Language：TypeScript
- Directory：当前目录

初始化完成后安装依赖：

```powershell
npm.cmd install
```

### 5.2 期望脚本
初始化后检查 `package.json`，至少应包含类似脚本：

```json
{
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "tauri": "tauri"
  }
}
```

若脚本名称与脚手架生成结果不同，以实际 `package.json` 为准。后续文档和 CI 应统一使用脚手架生成的脚本。

## 6. 初始化 Manim 开发环境
### 6.1 创建 Python 环境
在仓库根目录执行：

```powershell
uv init --bare
uv add manim
```

该环境用于开发期验证 Manim 渲染链路。它不是最终产品内置运行时方案。

### 6.2 检查 Manim 健康状态
执行：

```powershell
uv run --with manim manim checkhealth
```

若提示 LaTeX 缺失或不可用，检查 MiKTeX 是否已安装，并打开 MiKTeX Console 完成首次配置。Manim 渲染数学公式时依赖 LaTeX。

### 6.3 最小渲染验证
后续可添加一个临时 Manim 示例文件进行验证。示例文件应放在开发测试目录中，避免混入正式应用源码。

验证命令形态：

```powershell
uv run --with manim manim path\to\scene.py SceneName -ql
```

V1 默认产品输出为 720p/30fps；开发期可先使用 `-ql` 快速验证环境。

## 7. 开发验证流程
### 7.1 启动桌面开发服务
```powershell
npm.cmd run tauri dev
```

首次运行会触发 Rust 依赖下载和编译，耗时较长。若失败，优先检查 Rust、C++ Build Tools 和 WebView2。

### 7.2 前端构建验证
```powershell
npm.cmd run build
```

### 7.3 Rust 编译验证
```powershell
cd src-tauri
cargo check
cd ..
```

### 7.4 Manim 验证
```powershell
uv run --with manim manim checkhealth
```

初始化完成的最低验收条件：

- Tauri dev 窗口可启动。
- 前端构建通过。
- `cargo check` 通过。
- `uv run --with manim manim checkhealth` 不报告阻塞性错误。

## 8. Windows 常见问题
### 8.1 PowerShell 拦截 npm.ps1
现象：

```text
npm : 无法加载文件 ... npm.ps1，因为在此系统上禁止运行脚本
```

处理：

```powershell
npm.cmd -v
npm.cmd install
npm.cmd run tauri dev
```

本手册默认使用 `npm.cmd`，不要求修改全局 PowerShell 执行策略。

### 8.2 Git dubious ownership
现象：

```text
fatal: detected dubious ownership in repository
```

原因是当前 Windows 用户与仓库目录所有者不一致。若确认该目录可信，可手动执行：

```powershell
git config --global --add safe.directory E:/manim4learn
```

这会修改全局 Git 配置。不要在未确认目录可信时执行。

### 8.3 winget 无法运行
现象：

```text
winget.exe 无法运行: 系统无法访问此文件
```

处理：

- 从 Microsoft Store 更新“应用安装程序”（App Installer）。
- 重新打开 PowerShell。
- 若仍不可用，改用对应官网安装包。

### 8.4 WebView2 或 Build Tools 缺失
Tauri dev 启动或 Rust 编译失败时，优先检查：

- Microsoft Edge WebView2 Runtime 是否安装。
- Microsoft C++ Build Tools 是否安装。
- Visual Studio Build Tools 是否包含 C++ 桌面开发组件。
- Rust 工具链是否为 MSVC。

检查 Rust 工具链：

```powershell
rustup show
```

若不是 MSVC 工具链，安装：

```powershell
rustup toolchain install stable-x86_64-pc-windows-msvc
rustup default stable-x86_64-pc-windows-msvc
```

### 8.5 MiKTeX 缺包或首次配置
Manim 渲染公式时可能触发 MiKTeX 缺包。建议：

- 打开 MiKTeX Console。
- 完成首次初始化。
- 允许 MiKTeX 自动安装缺失包。
- 再次运行 `uv run manim checkhealth`。

## 9. 后续实施边界
本手册完成后，后续开发再进入应用实现阶段：

- Tauri 应用骨架整理。
- 首次启动工作区选择。
- Provider 配置与明文 Key 风险提示。
- OpenAI/Anthropic 兼容调用。
- Manim 渲染队列。
- MP4 预览。
- 生成历史与简化日志。

本手册不处理：

- 最终安装包打包。
- Python/Manim 托管运行时分发。
- 自动修复失败脚本。
- 多轮对话生成。
- 运行时质量评分。

## 10. 参考资料
- Tauri Windows 前置条件：https://v2.tauri.app/start/prerequisites/
- Tauri 创建项目：https://v2.tauri.app/start/create-project/
- Vite 开始指南：https://vite.dev/guide/
- Node.js 下载：https://nodejs.org/en/download
- uv 安装文档：https://docs.astral.sh/uv/getting-started/installation/
- Manim uv 安装指南：https://docs.manim.community/en/stable/installation/uv.html
- MiKTeX 下载：https://miktex.org/download
