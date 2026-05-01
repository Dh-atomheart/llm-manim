# 用户使用手册

本文面向安装 Release 版本的普通用户，说明如何从安装包启动 LLM-Manim，并完成一次 Manim 动画生成与渲染。

## 1. 安装应用

从 GitHub Releases 下载 Windows 安装包。通常会有两类文件：

- `*.msi`：Windows Installer 安装包。
- `*-setup.exe`：NSIS 安装器，更接近普通 Windows 软件安装体验。

安装包只包含 LLM-Manim 桌面应用本身。Manim 渲染所需的 Python、uv、FFmpeg、LaTeX/MiKTeX 需要在本机单独安装。完整步骤见 [Runtime 安装手册](runtime-installation.md)。

## 2. 准备 runtime

安装并验证以下命令可用：

```powershell
python --version
uv --version
uv run --with manim manim --version
ffmpeg -version
ffprobe -version
latex --version
dvisvgm --version
```

其中 `uv run --with manim manim --version` 最关键。LLM-Manim 渲染时使用 uv 托管 Manim，不要求用户全局安装 `manim`。

## 3. 首次启动与 workspace

首次启动应用时，需要选择 workspace。workspace 是你的本地工作目录，应用会在其中保存：

- `config/`：配置文件。
- `db/`：SQLite 数据库。
- `projects/`：项目相关数据。
- `jobs/`：生成任务的中间文件。
- `artifacts/`：渲染产物，例如 MP4。
- `logs/`：应用和任务日志。
- `temp/`：临时文件。
- `.runtime/`：静态检查临时资源与 runtime 检测辅助文件。

建议为每个长期使用的安装创建一个固定目录，例如：

```text
D:\LLM-Manim-Workspace
```

不要把 workspace 放在系统目录、只读目录或云同步冲突频繁的目录里。

## 4. 检查运行环境

在首次启动页面或基础设置页面点击环境检查。应用会检测：

- Python
- uv
- 全局 Manim CE（可选）
- uv 托管 Manim
- FFmpeg
- FFprobe
- LaTeX / MiKTeX
- dvisvgm

如果全局 Manim 缺失但 uv 托管 Manim 可用，通常可以继续使用。若 `uv`、`uv run --with manim manim --version`、FFmpeg、FFprobe、LaTeX 或 dvisvgm 缺失，公式渲染或视频输出可能失败。

## 5. 配置 Provider

进入 Provider 设置，新增一个模型 Provider。常用字段：

- 名称：方便识别，例如 `OpenAI`、`DeepSeek`、`Local Gateway`。
- 类型：`OpenAI-compatible` 或 `Anthropic-compatible`。
- Base URL：模型服务地址。
- Model：模型名。
- API Key：访问密钥。

保存前可以点击测试连接。更详细说明见 [Provider 配置手册](provider-configuration.md)。

## 6. 生成并渲染动画

在工作台中：

1. 选择项目和 Provider。
2. 输入动画需求，例如“用 Manim 展示二次函数图像和顶点移动过程”。
3. 提交任务。
4. 等待任务状态从 queued/running 变为 succeeded 或 failed。
5. 成功后在右侧预览 MP4，并可打开产物所在位置。

生成流程包含 LLM 调用、代码提取、静态检查、Manim 渲染和产物校验。任务失败时，应用会显示错误摘要和建议。

## 7. 历史、重试与删除

历史列表会保留已提交任务。常见操作：

- 刷新：重新读取任务列表和产物状态。
- 取消：对 queued/running 任务发起取消。
- 重试：基于失败或取消的任务重新创建新任务。
- 删除：软删除终态任务，queued/running 任务不能删除。
- 打开产物：用系统默认程序或文件管理器打开已生成 MP4。

## 8. 安全提醒

- API Key 只保存在本地配置中，但当前版本会提示明文存储风险。
- 不要把 workspace、日志或数据库直接公开上传。
- LLM 生成代码会经过静态检查，但仍建议检查最终视频内容是否符合预期。
