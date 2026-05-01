# LLM-Manim

LLM-Manim 是一个面向 Manim Community Edition 的桌面应用。它使用 LLM 根据用户提示生成 Manim 动画代码，在本地进行静态检查、渲染 MP4，并保存生成历史与日志，帮助用户更快制作数学、物理、算法和课程讲解类动画。

## 核心功能

- 通过 LLM 生成 ManimCE 动画代码。
- 对 LLM 生成的 Python/Manim 代码进行静态检查，降低不安全或不兼容代码进入渲染流程的风险。
- 调用本地 ManimCE runtime 渲染 MP4，并在应用中预览渲染结果。
- 使用 SQLite 保存 workspace、项目、Provider 配置、生成任务、渲染产物和日志。

## 演示视频

### 示例一

> prompt: 演示傅里叶级数如何用多个正弦波叠加逐步逼近一个方波。

<video src="docs\demo\FourierSquareWave.mp4" controls width="720"></video>

### 示例二

> prompt: 用节点和边的动画展示深度优先搜索在网格迷宫中的探索路径。

<video src="docs\demo\DFSMazeExploration.mp4" controls width="720"></video>

### 示例三

> prompt: 在3D场景中绘制一个双曲抛物面（马鞍面）z = x^2 - y^2。让相机围绕曲面旋转360度，同时用颜色映射表示高度。添加坐标轴和网格。

<video src="docs\demo\HyperbolicParaboloid.mp4" controls width="720"></video>

### 示例四

> prompt: 生成分形图案——科赫雪花。从一个等边三角形开始，迭代3次，每次将每条边替换为4条小边（科赫曲线构造）。显示每次迭代后的结果，并在角落标注迭代次数和边数变化。

<video src="docs\demo\KochSnowflake.mp4" controls width="720"></video>

### 示例五

> prompt: 模拟一个抛体运动：一个小球以初速度 v_0、角度 \theta 抛出，画出它的运动轨迹，实时显示速度矢量和加速度矢量。落地时要有轻微的弹跳效果，并在旁边显示运动方程。

<video src="docs\demo\ProjectileMotion.mp4" controls width="720"></video>

## 技术栈

- 桌面框架：Tauri 2
- 前端：React 19、TypeScript、Vite、Zustand、CSS Modules
- 后端：Rust、Tauri commands、Tokio、reqwest、serde
- 存储：SQLite、sqlx migrations
- 动画 runtime：Python、uv、Manim Community Edition、FFmpeg、LaTeX/MiKTeX
- 测试：Vitest、Playwright

## 普通用户快速安装

LLM-Manim V1 的安装包只包含桌面应用本身。Manim 渲染仍依赖本机命令行工具：Python、uv、FFmpeg/FFprobe、LaTeX/dvisvgm。即使你的电脑上完全没有这些工具，也可以按下面步骤安装。

### 1. 安装外部 runtime

打开 PowerShell，依次执行：

```powershell
winget install --id Python.Python.3.12 -e
powershell -ExecutionPolicy ByPass -c "irm https://astral.sh/uv/install.ps1 | iex"
winget install --id Gyan.FFmpeg -e
winget install --id MiKTeX.MiKTeX -e
```

安装完成后，关闭并重新打开 PowerShell。如果应用已经打开，也建议重启应用，让新安装的命令加入 `PATH` 后能被检测到。

相关官方文档：

- [Microsoft WinGet](https://learn.microsoft.com/windows/package-manager/winget/)
- [uv installation](https://docs.astral.sh/uv/getting-started/installation/)
- [Manim Community installation](https://docs.manim.community/en/stable/installation.html)
- [MiKTeX setup](https://miktex.org/howto/install-miktex)

### 2. 验证 runtime

在新的 PowerShell 窗口中执行：

```powershell
python --version
uv --version
uv run --with manim manim --version
ffmpeg -version
ffprobe -version
latex --version
dvisvgm --version
```

其中 `uv run --with manim manim --version` 是关键验证项。应用渲染时使用的也是 `uv run --with manim manim ...`，所以用户不需要全局安装 `manim`。第一次运行该命令时，uv 会下载并准备 Manim 运行环境，耗时较长是正常的。

应用界面里可能会显示“全局 Manim”状态；它只是可选检测项。只要 Python、uv、uv 托管的 Manim、FFmpeg、FFprobe、LaTeX 和 dvisvgm 都可用，就可以进行完整渲染。

### 3. LaTeX / MiKTeX 注意事项

Manim 的 `MathTex`、公式轴标签和部分 TeX 文本需要 LaTeX 与 `dvisvgm`。安装 MiKTeX 后，首次渲染公式时可能会提示安装缺失包；建议允许 MiKTeX 自动安装缺失包。

如果只生成纯图形或普通 `Text` 文本，LaTeX 的使用频率会低一些；但为了稳定渲染数学公式，仍建议安装 MiKTeX。

### 4. 首次启动应用

1. 选择或创建 workspace。
2. 检查 runtime 状态，确认缺失项已经安装。
3. 配置 Provider，支持 OpenAI-compatible 或 Anthropic-compatible。
4. 填写 `base_url`、`api_key` 和 `model`。
5. 输入提示词，生成并渲染 Manim 动画。

## 开发者快速开始

如果你要从源码运行或重新打包，请先安装 Node.js/npm、Rust stable toolchain 和 Tauri 2 所需系统依赖，然后在项目根目录执行：

```powershell
npm install
npm run tauri dev
```

常用命令：

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

## Manim 渲染方式

应用不会直接依赖全局 `manim` 命令进行渲染。当前渲染流程由 Rust 后端启动：

```text
uv run --with manim manim --config_file <manim.cfg> <generated_scene.py> <SceneName> -qm --format=mp4 --media_dir <media_dir>
```

这意味着：

- `uv` 必须能在系统 `PATH` 中被找到。
- `uv run --with manim manim --version` 必须能成功运行。
- 全局 `manim --version` 可以不存在。
- FFmpeg/FFprobe 负责视频处理和产物校验。
- LaTeX/dvisvgm 负责公式和 TeX 文本渲染。

## 项目结构

```text
src/                  React 前端、视图、组件、状态和 Tauri command client
src-tauri/            Tauri/Rust 后端、SQLite migrations、渲染与 Provider 服务
docs/                 架构、接口、测试、runtime、发布准备等项目文档
references/           构建期嵌入的 ManimCE manifest、规则、技能和模板
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
- 渲染依赖本地 Python、uv、ManimCE、FFmpeg 和 LaTeX runtime。
- 不要提交本地 runtime、构建产物、日志、API Key 或 workspace 私有数据。

## 文档导航

- [文档中心](docs/index.md)：按普通用户、开发者和维护者整理的手册入口。
- [用户使用手册](docs/user-guide.md)：安装应用、选择 workspace、配置 Provider、生成动画、查看历史与产物。
- [Runtime 安装手册](docs/runtime-installation.md)：Windows 从零安装 Python、uv、FFmpeg、MiKTeX，并验证渲染环境。
- [Provider 配置手册](docs/provider-configuration.md)：配置 OpenAI-compatible / Anthropic-compatible Provider 并排查连接问题。
- [开发者手册](docs/developer-guide.md)：源码运行、项目结构、前后端边界和测试命令。
- [发布与安装包手册](docs/release-packaging.md)：构建 MSI/NSIS 安装包和发布前检查。
- [References 打包说明](docs/references-packaging.md)：说明 `references/` 如何编译进二进制，以及 Release 用户为什么不需要该目录。
- [故障排查手册](docs/troubleshooting.md)：runtime、Provider、渲染、Cargo 缓存等常见问题。

更多使用、开发和发布细节见 [文档中心](docs/index.md)。

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.
