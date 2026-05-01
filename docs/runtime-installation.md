# Runtime 安装手册

本文说明如何在一台全新的 Windows 电脑上准备 LLM-Manim 所需的外部 runtime。

## 需要安装什么

LLM-Manim 安装包不内置 Manim 生态依赖。你需要安装：

- Python：供 uv 和 Manim 运行。
- uv：用于临时托管并运行 Manim。
- FFmpeg / FFprobe：用于视频输出和产物校验。
- MiKTeX / LaTeX：用于 `MathTex`、公式标签和 TeX 文本。
- dvisvgm：通常随 MiKTeX 安装，用于公式转 SVG。

## Windows 快速安装

打开 PowerShell，依次执行：

```powershell
winget install --id Python.Python.3.12 -e
powershell -ExecutionPolicy ByPass -c "irm https://astral.sh/uv/install.ps1 | iex"
winget install --id Gyan.FFmpeg -e
winget install --id MiKTeX.MiKTeX -e
```

安装完成后，关闭并重新打开 PowerShell。如果 LLM-Manim 已经打开，也请重启应用。

官方文档：

- Microsoft WinGet: https://learn.microsoft.com/windows/package-manager/winget/
- uv installation: https://docs.astral.sh/uv/getting-started/installation/
- Manim Community installation: https://docs.manim.community/en/stable/installation.html
- MiKTeX setup: https://miktex.org/howto/install-miktex

## 验证命令

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

每个命令都应该输出版本信息。如果提示“不是内部或外部命令”，通常表示命令没有加入 `PATH`，需要重启终端、重启应用，或重新安装对应工具。

## 为什么不要求全局安装 Manim

应用渲染时使用：

```text
uv run --with manim manim ...
```

这表示 uv 会为命令准备可用的 Manim 环境。应用界面可能仍显示“全局 Manim CE（可选）”，但它不是必要条件。真正必要的是 `uv run --with manim manim --version` 能成功运行。

第一次运行 uv 托管 Manim 时，uv 可能会下载依赖，耗时较长。之后会使用本地缓存，速度通常会变快。

## MiKTeX 和公式渲染

Manim 的 `MathTex` 和公式轴标签依赖 LaTeX 与 dvisvgm。首次渲染公式时，MiKTeX 可能弹出提示安装缺失包。建议允许自动安装缺失包，否则公式渲染可能失败。

如果你暂时不需要公式，只做几何图形、普通文字和动画，LaTeX 使用频率较低；但为了稳定使用 LLM-Manim，仍建议安装 MiKTeX。

## 常见安装问题

- `winget` 不存在：请确认 Windows App Installer 已安装，或改用对应工具官网下载安装包。
- `uv` 安装后命令不可用：重启 PowerShell；必要时检查用户目录下的 uv 安装路径是否加入 PATH。
- `ffmpeg` 可用但 `ffprobe` 不可用：重新安装 FFmpeg，确认安装包包含 FFprobe。
- `latex` 可用但 `dvisvgm` 不可用：检查 MiKTeX 是否完整安装，或通过 MiKTeX Console 安装缺失组件。
