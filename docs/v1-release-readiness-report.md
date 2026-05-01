# V1 发布就绪检测报告

## 结论

当前项目暂不满足 V1 正式发布标准。核心功能链路、自动化测试和安装包构建已经通过，但真实 Provider、真实 Manim runtime、真实安装包手工验证三个发布级验收点尚未闭环。

正式发布口径按“可交付 Windows 桌面安装包”判断，而不是仅源码可运行或开发环境可启动。

## 已完成检测

| 检测项 | 结果 | 说明 |
| --- | --- | --- |
| 前端单元测试 | 通过 | `npm test`，5/5 passed |
| 前端生产构建 | 通过 | `npm run build`，TypeScript 与 Vite build 通过 |
| Rust 测试 | 通过 | `cargo test`，52/52 passed |
| Playwright 验收 | 通过 | `npm run test:e2e`，4/4 passed |
| Tauri release 可执行文件 | 通过 | 已生成 `src-tauri/target/release/manim4learn.exe` |
| Tauri 安装包 | 通过 | 已生成 MSI 与 NSIS setup |

本轮验证时发现 C 盘可用空间为 0，导致 npm、MSVC linker 和最初的 Tauri bundle 阶段失败。已按发布修复路径清理仓库内可再生构建产物，并将本轮命令的 `TEMP`、`TMP`、`NPM_CONFIG_CACHE` 临时指向工作区 `.tmp` 后完成验证。正式发布机仍应保留充足的系统盘临时空间。

## 功能完整度矩阵

| V1 能力 | 当前状态 | 证据或缺口 |
| --- | --- | --- |
| 工作区初始化 | 基本完成 | 支持目录选择、标准目录创建、SQLite 初始化、runtime 检查 |
| Provider 配置 | 基本完成 | 支持 OpenAI-compatible 与 Anthropic-compatible；列表不回显 API Key |
| 项目管理 | 基本完成 | 支持项目创建、列表、删除 |
| 任务提交与队列 | 基本完成 | 默认串行队列，queued/running/succeeded/failed/cancelled 状态可追踪 |
| LLM 输出解析 | 基本完成 | 覆盖无 Markdown、多 Markdown、Provider 响应无效等后端测试 |
| 静态安全检查 | 基本完成 | 覆盖 ManimGL、危险 import、危险调用、文件/网络/命令副作用 |
| Manim 渲染 | 基本完成 | 覆盖成功、失败、依赖缺失、取消；真实 runtime 尚需验收 |
| Artifact 校验 | 基本完成 | 覆盖 MP4 缺失、fatal 日志、duration 校验、路径越界 |
| 视频预览与打开 | 基本完成 | 支持安全 asset URL 与文件管理器定位 |
| 日志脱敏 | 基本完成 | 覆盖数据库日志、文件日志、Provider 错误详情脱敏 |
| 取消与重试 | 基本完成 | 覆盖 queued/running 取消与 failed/cancelled 重试 |
| Golden prompts | 未闭环 | `docs/golden-prompts-manual-acceptance.md` 仍为待填写 |
| 安装包交付 | 构建完成，手工验证未闭环 | MSI 与 NSIS setup 已生成，安装/卸载尚未人工确认 |

## 发布阻塞项

1. `docs/golden-prompts-manual-acceptance.md` 的四类真实 golden prompt 结果仍未填写，不能证明真实 Provider + 真实 Manim runtime 可稳定产出合格 MP4。
2. 安装包手工验证尚未执行：安装、启动、初始化工作区、Provider 测试、创建项目、提交任务、视频预览、打开 artifact、取消、重试、卸载。
3. M6 的部分发布级证据仍缺失：数据库迁移失败回滚、Windows 取消压力测试、全部失败场景 UI 层回归、真实日志/API Key 脱敏抽检。
4. 发布环境仍有系统盘空间风险：本机 C 盘可用空间为 0，正式构建机必须预留足够临时空间，或显式配置构建临时目录到容量充足的磁盘。

## 已执行修复

1. 发布版本号统一提升到 `1.0.0`：
   - `package.json`
   - `package-lock.json`
   - `src-tauri/Cargo.toml`
   - `src-tauri/tauri.conf.json`
2. Tauri bundle identifier 从 `com.manim4learn.app` 调整为 `com.manim4learn.desktop`，避免以 `.app` 结尾。
3. Tauri CSP 从 `null` 改为最小发布策略，允许应用自身资源、Tauri IPC、asset/video 预览与必要的内联样式。
4. 已清理仓库内可再生构建产物并重跑 `npm run tauri -- build`，成功产出：
   - `src-tauri/target/release/bundle/msi/LLM-Manim V1_1.0.0_x64_en-US.msi`
   - `src-tauri/target/release/bundle/nsis/LLM-Manim V1_1.0.0_x64-setup.exe`

## 修复路径

### 1. 发布包修复

- 正式构建机清理或扩容系统盘临时空间；本机当前 C 盘为 0 可用，不适合作为无额外环境变量的发布构建机。
- 如短期无法释放 C 盘，可在构建命令中设置 `TEMP`、`TMP`、`NPM_CONFIG_CACHE` 到容量充足的磁盘。
- 已重新运行 `npm run tauri -- build` 并产出 MSI/NSIS 安装包。
- 仍需手工验证 Windows 安装包安装、启动、卸载均可用。

### 2. Golden Prompts 真实验收

- 在真实 Provider、真实 API Key、真实 `uv/manim/ffmpeg/ffprobe` 环境中运行四类 prompts。
- 每类至少记录一次完整结果：Job ID、artifact 路径、duration、文件大小、日志是否含 fatal、人工观察结论。
- 将结果写回 `docs/golden-prompts-manual-acceptance.md`。
- 若失败，补充错误码、UI 错误摘要、是否可重试、重试结果、是否泄露敏感信息。

### 3. M6 验收补齐

- 增加数据库迁移失败回滚验证，确认迁移失败后不会继续写业务数据。
- 做 Windows running 任务取消压力测试，至少连续 5 次长任务取消后队列仍可继续处理。
- 扩展 UI 层验收覆盖网络超时、Provider 响应结构无效、LLM 无 Markdown、多 Markdown、Manim 渲染失败、artifact 缺失或 duration 为 0。
- 抽检 `logs/app.log`、SQLite `job_logs`、UI 错误区域，确认假 API Key 不出现。

### 4. 发布前配置复核

- 确认 `src-tauri/capabilities/default.json` 仍只保留必要权限。
- 确认前端只通过 `src/commands/*` 调用 Tauri command，不绕过后端访问 Provider、文件或 shell。
- 确认 UI 不展示、不复制、不导出生成的 Manim 源码。

## 发布前最终验收清单

- [x] `npm test`
- [x] `npm run build`
- [x] `cargo test`
- [x] `npm run test:e2e`
- [x] `npm run tauri -- build`
- [ ] Windows 安装包安装验证
- [ ] Windows 安装包卸载验证
- [ ] 四类真实 golden prompt 验收完成并记录
- [ ] 真实日志/API Key 脱敏抽检通过
- [ ] Windows running 任务取消压力测试通过
- [ ] 数据库迁移失败回滚验证通过

## 当前发布建议

保持 V1 release candidate 状态。待真实 golden prompt、手工安装包验收和剩余 M6 证据全部闭环后，再标记为 V1 正式发布。
