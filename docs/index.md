# LLM-Manim 文档中心

这里收集 LLM-Manim 的用户手册、开发手册和发布维护说明。普通用户可以从安装和使用流程开始；开发者可以直接阅读源码运行、构建发布和 references 打包说明。

## 普通用户

- [用户使用手册](user-guide.md)：从安装包安装、选择 workspace、配置 Provider、提交提示词、查看历史与渲染产物。
- [Runtime 安装手册](runtime-installation.md)：在 Windows 上从零安装 Python、uv、FFmpeg、MiKTeX，并验证 Manim 渲染环境。
- [Provider 配置手册](provider-configuration.md)：配置 OpenAI-compatible / Anthropic-compatible Provider、测试连接、排查 API 错误。

## 开发者

- [开发者手册](developer-guide.md)：本地源码运行、项目结构、前后端边界、常用测试和构建命令。
- [发布与安装包手册](release-packaging.md)：构建 MSI/NSIS 安装包、发布前检查、目录重命名后的缓存处理。
- [References 打包说明](references-packaging.md)：解释 `references/` 如何在构建期嵌入二进制，以及 Release 用户为什么不需要该目录。

## 维护与排错

- [故障排查手册](troubleshooting.md)：runtime 缺失、uv 首次下载慢、MiKTeX 缺包、FFmpeg PATH、Provider 401/超时、Cargo 旧路径缓存等问题。

## 推荐阅读路径

首次使用：先读 [Runtime 安装手册](runtime-installation.md)，再读 [用户使用手册](user-guide.md)。

准备发布：先读 [发布与安装包手册](release-packaging.md)，再读 [References 打包说明](references-packaging.md)。

参与开发：先读 [开发者手册](developer-guide.md)，再按需阅读 [Provider 配置手册](provider-configuration.md) 和 [故障排查手册](troubleshooting.md)。
