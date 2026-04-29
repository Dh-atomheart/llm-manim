# LLM-Manim V1 LLM 编排规格

## 1. 目标
本文定义 V1 的 LLM 工作流如何由 Rust/Tauri 后端编排。V1 不使用 LangChain、LangGraph 或类似 agent 框架；实现者应在后端实现轻量 `LLM Orchestrator Service`。

## 2. 总流程
```text
submit_prompt_job
-> create PromptJob(queued)
-> dequeue and set running
-> load PromptJob and ProviderConfig
-> build prompt
-> call provider
-> extract model text
-> parse single Markdown Python code block
-> static check
-> handoff to render execution
-> persist result or error
```

任意阶段失败：

```text
write JobLog
-> set error_code/error_summary/suggestion
-> set PromptJob(failed)
-> stop before render if code is not validated
```

## 3. LLM Orchestrator Service 职责
负责：

- 读取 queued/running `PromptJob`。
- 读取关联 `ProviderConfig`。
- 调用 Prompt/Skill Service 构建固定 Prompt。
- 调用 Provider Service 获取模型文本输出。
- 提取 Provider 返回的文本内容。
- 解析唯一 Markdown Python 代码块。
- 调用 Static Checker 校验代码。
- 将通过校验的代码和 `SceneName` 交给 Render Execution Layer。
- 写入阶段日志、错误码、错误摘要和建议动作。

不负责：

- 不直接执行 shell。
- 不写任意路径。
- 不调用 Manim、uv 或 FFmpeg。
- 不自动修复失败脚本。
- 不做多轮对话。
- 不向用户展示或导出 Manim 源码。
- 不绕过 Provider、Prompt、Static Checker 或 Render 层的职责边界。

## 4. 服务边界
### 4.1 Provider Service
Provider Service 只负责：

- 按 [provider-protocol.md](provider-protocol.md) 发送 HTTP 请求。
- 解析 Provider 响应为模型文本。
- 映射 Provider 错误。
- 脱敏 Provider 日志。

Provider Service 不解析 Markdown 代码块，不执行工具，不决定任务终态。

### 4.2 Prompt/Skill Service
Prompt/Skill Service 只负责：

- 按 [prompt-contract.md](prompt-contract.md) 构建 system/user prompt。
- 注入 ManimCE 规则。
- 固定输出合同。

Prompt/Skill Service 不调用模型，不处理 Provider 错误。

### 4.3 Static Checker
Static Checker 只负责：

- 按 [static-checker.md](static-checker.md) 校验提取后的 Python 代码。
- 返回 `SceneName` 或 `E_STATIC_CHECK_FAILED`。

Static Checker 不调用 Provider，不运行 Manim。

### 4.4 Render Execution Layer
Render Execution Layer 只接收已通过静态校验的代码和 `SceneName`。

Render Execution Layer 不接收模型原始文本，不解析 Markdown，不信任模型指定的命令、路径或输出目录。

## 5. 阶段日志
Orchestrator 至少写入以下阶段日志：

- `prompt build started`
- `prompt build finished`
- `provider request started`
- `provider response received`
- `model text extracted`
- `markdown parse started`
- `markdown parse finished`
- `static check started`
- `static check finished`
- `handoff to render`
- `orchestrator failed`

日志不得包含：

- API Key。
- Authorization header。
- 完整 Provider secret。
- 未脱敏 Provider 原始请求体。
- 普通用户界面可见的完整 Manim 源码。

## 6. 错误映射
```text
Provider 鉴权失败 -> E_AUTH_401
Provider 超时 -> E_NET_TIMEOUT
Provider 非 2xx 或服务错误 -> E_PROVIDER_ERROR
Provider 响应结构无效 -> E_PROVIDER_RESPONSE_INVALID
无 Markdown 代码块 -> E_LLM_OUTPUT_INVALID
多个 Markdown 代码块 -> E_LLM_OUTPUT_INVALID
Python 代码结构或安全校验失败 -> E_STATIC_CHECK_FAILED
用户取消 -> E_CANCELLED
```

失败时必须写入：

- `PromptJob.state = failed` 或 `cancelled`。
- `error_code`。
- `error_summary`。
- `suggestion`。
- 对应 `JobLog`。

## 7. 取消处理
- Provider 请求前取消：直接标记 `cancelled`。
- Provider 请求中取消：尽力中止请求或忽略响应，标记 `cancelled`。
- 静态校验前取消：不写入可执行渲染脚本。
- 已交给 Render Execution 后取消：由 Render Queue/Render Execution 按 [render-pipeline.md](render-pipeline.md) 处理。

## 8. 为什么 V1 不使用 LangChain/LangGraph
V1 不引入 LangChain、LangGraph 或类似 agent 框架。

原因：

- V1 是单轮固定流程，不是多轮 agent。
- V1 不做自动修复、工具规划或多步骤模型决策。
- Provider 调用已由 Rust `reqwest` 和 [provider-protocol.md](provider-protocol.md) 固定。
- Prompt 输出合同已由 [prompt-contract.md](prompt-contract.md) 固定。
- 本地副作用必须由 Rust/Tauri 后端控制，agent 框架容易弱化“模型只请求，应用执行”的边界。
- 引入 Python/JS 编排层会增加 Windows 桌面应用的运行时复杂度。

## 9. V2 可重新评估的条件
后续版本出现以下需求时，可重新评估 LangChain、LangGraph 或其他 agent/workflow 框架：

- 多轮对话生成。
- 自动修复失败脚本。
- 多工具规划。
- 多 Provider 路由。
- 可解释中间步骤。
- 用户可见的生成计划和修改回合。

## 10. 验收标准
- mock Provider 返回有效 ManimCE 代码块时，Orchestrator 能进入静态校验并交给渲染阶段。
- 无代码块或多代码块返回 `E_LLM_OUTPUT_INVALID`。
- ManimGL 或危险 API 返回 `E_STATIC_CHECK_FAILED`。
- Provider 鉴权失败返回 `E_AUTH_401`。
- Provider 响应结构无效返回 `E_PROVIDER_RESPONSE_INVALID`。
- 任意失败都不会进入 Manim 渲染，并写入用户可读错误摘要和建议动作。
