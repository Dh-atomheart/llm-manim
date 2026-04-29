# LLM-Manim V1 Runtime 管理规格

## 1. 目标
本文定义工作区内半托管 `.runtime` 的创建、检查、修复、版本锁定和 Manim 子进程管理。

## 2. Runtime 策略
V1 使用工作区内半托管 uv runtime：

```text
.runtime/
  uv/
  python/
  manim/
  checks/
  locks/
```

V1 不承诺：

- 完整离线 Python/Manim/MiKTeX 打包。
- macOS/Linux runtime 分发。
- CPU/内存硬隔离。

## 3. 版本锁定
首次创建 runtime 时必须记录：

- Python 版本。
- uv 版本。
- Manim Community Edition 版本。
- FFmpeg 版本。
- LaTeX/MiKTeX 状态。
- 依赖锁定文件或等效版本记录。

建议策略：

- 使用 uv 管理 Python 环境。
- Manim 使用明确版本，不使用无约束最新版。
- runtime 检查结果写入日志和 SQLite 状态摘要。

## 4. 检查项
`check_runtime` 必须检查：

- 工作区可读写。
- `.runtime` 是否存在。
- uv 是否可用。
- Python 是否可用。
- ManimCE 是否可 import。
- `manim checkhealth` 是否通过阻塞项。
- FFmpeg 是否可用。
- LaTeX/MiKTeX 是否可用或给出公式渲染风险提示。

状态：

```text
ready | missing | invalid | repairing
```

## 5. 修复策略
V1 提供修复引导，不强制全自动修复。

允许的修复动作：

- 创建缺失目录。
- 初始化 uv 环境。
- 安装锁定版本的 Manim。
- 重新运行健康检查。

必须提示用户的情况：

- 需要安装系统级依赖。
- 需要 MiKTeX/LaTeX。
- 需要网络下载依赖。
- 当前工作区不可写。

## 6. 渲染进程管理
Manim 渲染由 Rust 后端使用 `tokio::process` 启动。

要求：

- command 模板由应用固定生成。
- 工作目录由后端固定。
- stdout/stderr 流式读取并写入日志文件。
- 进程句柄必须和 running job 关联。
- 用户取消时终止对应子进程。
- 进程结束后必须检查 exit code。

前端、模型和用户提示词不得影响：

- 可执行文件。
- 参数列表。
- 工作目录。
- 输出目录。
- 日志路径。

## 7. 崩溃恢复
应用启动时应检查 SQLite 中的 running job：

- 若没有对应进程，标记为 failed 或 cancelled，并写恢复日志。
- 若临时文件缺失，标记为 failed。
- 不自动重新渲染。

## 8. 硬质量检查依赖
runtime 层必须提供读取 MP4 duration 的能力。可通过 FFmpeg/ffprobe 或受控媒体探测方式实现。

读取失败映射为 `E_ARTIFACT_INVALID`。

## 9. 验收标准
- 缺 uv 时返回 `E_DEP_MISSING`。
- 缺 ManimCE 时返回 `E_DEP_MISSING`。
- ManimGL 不被 runtime 接受为有效环境。
- running 渲染可取消。
- 子进程非零退出映射为 `E_RENDER_FAIL`。
- 应用崩溃后重启不会保留无法解释的 running 状态。
