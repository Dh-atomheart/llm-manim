# 发布与安装包手册

本文说明如何在本地构建 LLM-Manim 的 Windows 安装包，并进行发布前检查。

## 发布前检查

建议按顺序执行：

```powershell
npm run build
cargo check
npm test
npm run test:e2e
```

如果只做文档变更，`npm run build` 和 `cargo check` 通常足够。涉及 UI 或任务流程时，应补充单元测试和 e2e 测试。

## 构建安装包

在仓库根目录执行：

```powershell
npm run tauri -- build
```

Tauri 会先执行 `beforeBuildCommand`，也就是：

```powershell
npm run build
```

构建成功后，Windows 安装包通常输出到：

```text
src-tauri/target/release/bundle/msi/
src-tauri/target/release/bundle/nsis/
```

常见文件类型：

- `*.msi`
- `*-setup.exe`

## 安装包包含什么

安装包包含：

- Tauri 桌面应用二进制。
- 前端构建产物。
- 应用图标和 Tauri bundle 资源。
- 编译期嵌入的 `references/` 关键内容。

安装包不包含：

- Python
- uv
- FFmpeg / FFprobe
- MiKTeX / LaTeX / dvisvgm
- 用户 workspace 数据
- 源码目录中的完整 `references/` 文件夹

普通用户安装后仍需按 [Runtime 安装手册](runtime-installation.md) 准备外部 runtime。

## GitHub Release 建议

发布 Release 时建议上传：

- NSIS `*-setup.exe`
- MSI `*.msi`
- 简短 Release Notes
- 已知限制：未签名安装包可能触发 Windows SmartScreen 提示；外部 runtime 需要用户自行安装。

## 目录重命名后的 Cargo 缓存

如果仓库目录从旧路径改名，例如从 `F:\manim4learn` 改为 `F:\llm-manim`，`src-tauri/target` 里可能残留旧绝对路径，导致 `cargo check` 或 Tauri build 报错。

解决方式：

```powershell
cd src-tauri
cargo clean
cargo check
```

`cargo clean` 会删除 Cargo 构建缓存，重新检查或构建会耗时更久，但可以清理旧路径问题。

## 发布前不要提交的内容

确认以下内容不会进入 Git：

- `node_modules/`
- `dist/`
- `src-tauri/target/`
- 本地 workspace
- `.runtime/`
- 日志、API Key、数据库和渲染产物

`references/` 不属于这些内容；它是源码构建必需目录，应保留在仓库中。
