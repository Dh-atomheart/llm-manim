# LLM-Manim V1 日志与可观测性规格

## 1. 目标
本文统一 V1 日志级别、阶段、落点、脱敏、用户日志与内部日志边界。分散在 Provider、LLM、渲染和存储文档中的日志要求以本文汇总执行。

## 2. 日志级别
```text
debug | info | warn | error
```

规则：

- 普通用户默认看到 `info/warn/error` 的简化内容。
- `debug` 仅用于开发者详情或内部文件日志。
- `error` 必须关联错误码或失败阶段。

## 3. 阶段枚举
```text
workspace
provider
prompt
llm
parse
static_check
queue
render
artifact
runtime
user_action
security
```

阶段名必须与 `JobLog.stage`、API 响应和前端显示保持一致。

## 4. 日志落点
- SQLite `job_logs`：任务阶段日志索引和用户可读摘要。
- `logs/app.log`：应用级内部日志。
- `jobs/{job_id}/render_stdout.log`：渲染 stdout。
- `jobs/{job_id}/render_stderr.log`：渲染 stderr。
- Provider 原始响应只允许保存脱敏摘要或截断片段。

## 5. 用户日志与内部日志
用户日志展示：

- 阶段。
- 状态。
- 简短消息。
- 错误码。
- 原因、影响、建议动作。

用户日志不得展示：

- API Key。
- Authorization header。
- Provider secret。
- 完整 Manim 源码。
- 任意敏感本地绝对路径。

内部日志可更详细，但仍必须脱敏。

## 6. 脱敏规则
写入任何日志前必须脱敏：

- API Key 明文。
- `Authorization: Bearer ...`
- `x-api-key`
- `api_key`
- `secret`
- Provider 请求体中的 secret 字段。

脱敏格式：

```text
[REDACTED]
```

## 7. 错误摘要生成
失败必须生成：

- `error_code`
- `error_summary`
- `suggestion`

摘要原则：

- 面向教师/教研用户。
- 使用“原因 + 影响 + 建议动作”。
- 不展示源码。
- 不要求用户理解 Python traceback。

## 8. 里程碑映射
- M3：Provider 连接测试日志和 API Key 脱敏。
- M4：LLM、解析、静态校验、渲染、artifact 阶段日志。
- M5：前端简化日志面板和开发者详情折叠。
- M6：日志脱敏回归测试。

## 9. 验收标准
- 假 API Key 不出现在 SQLite、文件日志、前端日志和错误 details。
- Provider 错误、LLM 输出错误、静态校验错误、渲染错误都有阶段日志。
- 用户界面能看到可读错误摘要和建议动作。
- stdout/stderr 写入前或展示前经过通用脱敏器。
