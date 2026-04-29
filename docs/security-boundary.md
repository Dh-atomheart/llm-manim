# LLM-Manim V1 安全边界规格

## 1. 目标
本文集中定义 V1 的安全边界和已知风险。V1 会执行由模型生成的 Manim 代码，因此必须默认模型输出不可信。

## 2. 核心原则
- 模型输出不可信。
- 前端不执行命令。
- 前端不直接访问文件系统。
- 本地副作用只能通过白名单 Tauri command 进入 Rust 后端。
- Rust 后端是状态、文件、进程、日志和路径校验的唯一权威。
- 静态校验是必要防线，但不是完整沙箱。

## 3. 模型输出边界
LLM 输出必须经过：

```text
Markdown code block parse
-> Python AST static check
-> denylist scan
-> controlled job script write
-> fixed Manim command
```

失败时：

- 不得写入可执行渲染脚本。
- 不得进入 Manim 渲染。
- 任务必须 failed，并写入可读错误。

## 4. Command 边界
所有 Tauri command 必须：

- 校验参数。
- 返回统一 `{ ok, data, error }`。
- 不接受前端传入的 shell 命令。
- 不接受任意本地路径作为执行依据。
- 不返回 API Key。

## 5. 路径边界
- 所有项目、任务、日志、artifact 路径必须位于 workspace 内。
- 数据库中优先保存 workspace 相对路径。
- `open_render_artifact` 必须先校验 artifact 属于 workspace。
- 模型输出不得影响输出目录、文件名或 media_dir。

## 6. API Key 风险
V1 决策：API Key 明文保存在本机 workspace/配置中。

必须做到：

- 保存前明确提示风险。
- 列表接口不返回 Key。
- 日志和错误 details 不包含 Key。
- 前端 store 不持久化 Key。

## 7. 已知不提供
V1 不提供：

- 强沙箱。
- CPU/内存硬隔离。
- 完整离线 runtime 打包。
- 自动安全修复。
- 内容合规拦截。
- 云端账号和权限体系。

这些是已知风险，不得在 UI 或文档中暗示 V1 已完全隔离不可信代码。

## 8. 里程碑映射
- M1：建立前后端边界，前端不直接执行命令。
- M2：workspace 路径校验和删除范围限制。
- M3：API Key 明文风险、列表不返回 Key、日志脱敏。
- M4：模型输出解析、静态校验、固定渲染命令。
- M5：前端不展示源码、Key 或绝对路径。
- M6：安全边界回归测试。

## 9. 验收标准
- 危险 API 代码不会进入渲染。
- ManimGL 代码被拒绝。
- 任意 artifact 打开动作都经过后端校验。
- 假 API Key 不出现在 UI、日志、错误 details、测试快照中。
- 前端没有直接 Provider 调用、shell 调用或文件系统访问。
