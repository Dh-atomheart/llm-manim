# 故障排查手册

本文收集 LLM-Manim 常见问题和处理方式。

## Runtime 缺失

现象：应用环境检查显示 missing 或 broken。

处理：按 [Runtime 安装手册](runtime-installation.md) 安装并验证：

```powershell
python --version
uv --version
uv run --with manim manim --version
ffmpeg -version
ffprobe -version
latex --version
dvisvgm --version
```

安装后请重启 PowerShell 和 LLM-Manim。

## uv 托管 Manim 第一次很慢

现象：第一次执行 `uv run --with manim manim --version` 或第一次渲染等待很久。

原因：uv 需要下载并准备 Manim 及其 Python 依赖。

处理：保持网络可用并等待完成。后续运行会使用缓存，通常更快。

## 全局 Manim 缺失

现象：界面显示全局 Manim CE 缺失。

说明：全局 Manim 是可选项。只要 `uv run --with manim manim --version` 可用，应用仍可渲染。

## FFmpeg 或 FFprobe 不在 PATH

现象：`ffmpeg -version` 或 `ffprobe -version` 失败。

处理：重新安装 FFmpeg，或确认安装目录已加入系统 `PATH`。使用 winget 安装时可执行：

```powershell
winget install --id Gyan.FFmpeg -e
```

安装后重启终端和应用。

## MiKTeX 缺包或公式渲染失败

现象：任务失败，日志中出现 LaTeX、dvisvgm、`latex error converting to dvi` 等信息。

处理：

- 确认 `latex --version` 和 `dvisvgm --version` 可用。
- 打开 MiKTeX Console，允许自动安装缺失包。
- 重新运行失败任务或点击重试。

## Provider 401 或鉴权失败

现象：测试 Provider 或提交任务时出现 unauthorized、401、authentication 等错误。

处理：

- 检查 API Key 是否正确。
- 检查账号是否有对应模型权限。
- 检查 Provider 类型是否选对。
- 检查 base URL 是否属于该服务。

## Provider 超时

现象：请求长时间无响应或 timeout。

处理：

- 检查网络、代理或防火墙。
- 换一个较快模型测试。
- 确认 base URL 能从当前电脑访问。

## Provider 返回格式无效

现象：invalid response 或类似错误。

处理：通常是 Provider 类型与服务协议不匹配。OpenAI-compatible 服务应选择 OpenAI-compatible；Anthropic 风格服务应选择 Anthropic-compatible。

## 生成代码静态检查失败

现象：任务失败，错误指向 Manim API、denylist、ManimGL、文件访问、subprocess、shell command 等。

说明：应用会拒绝不安全或不兼容 ManimCE 的代码。

处理：

- 修改提示词，明确要求使用 Manim Community Edition。
- 避免要求读写本地文件、访问网络、执行命令或安装包。
- 重试任务。

## 渲染超时

现象：任务长时间 running 后失败，提示 render timeout。

处理：

- 简化提示词，减少对象数量、3D 元素或复杂公式。
- 避免要求过长动画。
- 检查电脑性能和 Manim 依赖是否正常。

## Cargo 报旧路径错误

现象：仓库目录重命名后，`cargo check` 报旧路径下的 Tauri permission 或 build artifact 文件不存在。

处理：

```powershell
cd src-tauri
cargo clean
cargo check
```

这是 Cargo/Tauri 构建缓存中的旧绝对路径导致的。

## references 缺失导致编译失败

现象：`cargo check` 或 Tauri build 报 `couldn't read references/...`。

处理：确认源码仓库中保留 `references/` 目录。Release 安装包用户不需要该目录，但开发者源码构建必须保留它。
