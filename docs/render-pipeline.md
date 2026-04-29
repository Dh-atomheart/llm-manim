# LLM-Manim V1 渲染流水线规格

## 1. 目标
本文定义从用户提示词到 MP4 产物的唯一 V1 流水线，包括 LLM 输出解析、静态校验、任务状态机、Manim 渲染、取消、手动重试和硬质量检查。

## 2. 流水线总览
```text
submit_prompt_job
-> create PromptJob(queued)
-> dequeue
-> set running
-> LLM Orchestrator: build prompt
-> LLM Orchestrator: call provider
-> LLM Orchestrator: parse Markdown Python code block
-> LLM Orchestrator: static check
-> write generated_scene.py
-> run ManimCE
-> hard artifact check
-> create RenderArtifact
-> set succeeded
```

任意阶段失败：

```text
write JobLog
-> set error_code/error_summary/suggestion
-> set failed
```

## 3. Prompt 编排
详细 LLM 编排见 [llm-orchestration.md](llm-orchestration.md)。详细 Prompt 模板、变量和输出合同见 [prompt-contract.md](prompt-contract.md)。本节只保留流水线约束。

Prompt 必须包含：

- V1 范围：单个 ManimCE Scene，输出 MP4。
- 版本约束：只使用 Manim Community Edition。
- 输出约束：只返回一个 Python Markdown 代码块。
- 安全约束：不得输出 Shell 命令、路径操作、网络请求、文件读写、子进程调用。
- 质量约束：布局居中、避免遮挡、逐步揭示、节奏清晰。

默认 skill 来源：

- `references/skills/manim-composer`
- `references/skills/manimce-best-practices`

禁止默认注入：

- `references/skills/manimgl-best-practices`

## 4. LLM 输出协议
Provider 调用、模型文本提取、Markdown 解析和静态校验的串联由 LLM Orchestrator Service 负责，见 [llm-orchestration.md](llm-orchestration.md)。

V1 使用 Markdown 代码块，不强制 JSON Schema。

有效输出示例：

````markdown
```python
from manim import *

class GeneratedScene(Scene):
    def construct(self):
        title = Text("二次方程求根公式")
        self.play(Write(title))
        self.wait(1)
```
````

解析规则：

- 必须存在且只存在一个 Python 代码块。
- 允许代码块语言为 `python`、`py` 或空；推荐 `python`。
- 若存在多个代码块，返回 `E_LLM_OUTPUT_INVALID`。
- 若没有代码块，返回 `E_LLM_OUTPUT_INVALID`。
- 代码块外文本全部忽略，但可记录到开发日志。
- 提取后的代码进入静态校验；不得直接写入可执行位置。

## 5. 静态校验
详细 AST 校验、denylist、SceneName 提取和测试用例见 [static-checker.md](static-checker.md)。本节只定义流水线最低约束。

### 5.1 必须满足
- 包含 `from manim import *`。
- 至少定义一个继承 `Scene`、`MovingCameraScene` 或 `ThreeDScene` 的类。
- V1 只渲染一个 Scene；若有多个 Scene，选择失败并返回 `E_STATIC_CHECK_FAILED`。
- Scene 类名必须可由应用识别并用于渲染命令。

### 5.2 必须拒绝
- `from manimlib import *`
- `import manimlib`
- `InteractiveScene`
- `manimgl`
- `os.system`
- `subprocess`
- `socket`
- `requests`
- `urllib`
- `open(`
- `Path(`
- `eval`
- `exec`
- `compile`
- `__import__`
- `input(`

### 5.3 限制
- 代码长度超过配置阈值时返回 `E_STATIC_CHECK_FAILED`。
- 禁止模型指定输出路径、媒体目录或 shell 命令。
- 禁止显式删除文件或遍历目录。

### 5.4 实现要求
V1 使用 Python AST 结构检查，并叠加 denylist 扫描。仅字符串扫描不得作为完整 V1 静态校验实现。

## 6. 渲染目录
每个任务使用独立目录：

```text
jobs/{job_id}/
  generated_scene.py
  render_stdout.log
  render_stderr.log
  manim.cfg
artifacts/{project_id}/{job_id}/
  output.mp4
```

规则：

- `generated_scene.py` 只在静态校验通过后写入。
- `output.mp4` 必须写入标准 artifact 目录。
- 模型不得影响文件名、目录名或输出位置。

## 7. 渲染命令
runtime 创建、版本锁定、健康检查和子进程管理见 [runtime-management.md](runtime-management.md)。

命令由应用固定生成。模板：

```text
uv run manim jobs/{job_id}/generated_scene.py {SceneName} -qm --format=mp4 --media_dir artifacts/{project_id}/{job_id}/media
```

V1 产品默认目标为 720p/30fps。若 Manim quality flag 与 720p/30fps 不能完全一致，应通过 `manim.cfg` 固定：

```ini
[CLI]
quality = medium_quality
format = mp4
frame_rate = 30
pixel_height = 720
pixel_width = 1280
```

规则：

- 前端不得传渲染命令。
- 模型不得传渲染命令。
- 用户 V1 不配置分辨率和 FPS。
- stdout/stderr 必须写入任务日志文件，并抽取摘要写入 `job_logs`。

## 8. 状态机
```text
queued -> running -> succeeded
queued -> running -> failed
queued -> cancelled
running -> cancelled
failed -> queued (retry creates new job)
```

阶段映射：

- queued：任务已入库，等待串行队列。
- running：Provider 调用、解析、校验、渲染或检查中。
- succeeded：MP4 通过硬质量检查并入库。
- failed：任意不可恢复错误。
- cancelled：用户取消 queued/running 任务。

## 9. 取消策略
### 9.1 queued
- 直接标记 `cancelled`。
- 写入 `JobLog(user_action, info)`。

### 9.2 running before render process
- 设置取消标记。
- 当前阶段尽快停止。
- 标记 `cancelled`。

### 9.3 running during render process
- 终止 Manim 子进程。
- 等待进程退出。
- 写入 stdout/stderr 摘要。
- 标记 `cancelled`。

取消失败：

- 返回 `E_CANCEL_FAILED`。
- 若进程仍在运行，任务保持 running，并写错误日志。

## 10. 手动重试
`retry_job` 不修改原任务，而是创建新任务：

- 复制 `project_id`。
- 复制 `provider_id`。
- 复制 `prompt_text`。
- 设置 `retry_of_job_id`。
- 状态为 `queued`。

允许重试：

- failed
- cancelled

不允许重试：

- queued
- running
- succeeded

## 11. 硬质量检查
V1 成功必须满足：

- MP4 文件存在。
- 文件大小大于最小阈值。
- 文件可读取 duration。
- duration > 0。
- Manim 日志无 fatal 错误。

失败映射：

- 文件不存在：`E_ARTIFACT_INVALID`
- 文件为空或过小：`E_ARTIFACT_INVALID`
- duration 读取失败：`E_ARTIFACT_INVALID`
- Manim 进程非零退出：`E_RENDER_FAIL`

V1 不做：

- 抽帧视觉评分。
- 遮挡自动判断。
- 教学质量自动评分。
- 自动修复。

## 12. 错误映射
```text
Provider 鉴权失败 -> E_AUTH_401
Provider 超时 -> E_NET_TIMEOUT
Provider 其他错误 -> E_PROVIDER_ERROR
Provider 响应结构无效 -> E_PROVIDER_RESPONSE_INVALID
无 Markdown 代码块 -> E_LLM_OUTPUT_INVALID
多个 Markdown 代码块 -> E_LLM_OUTPUT_INVALID
ManimGL 或危险 API -> E_STATIC_CHECK_FAILED
依赖缺失 -> E_DEP_MISSING
Manim 非零退出 -> E_RENDER_FAIL
MP4 硬质量失败 -> E_ARTIFACT_INVALID
用户取消 -> E_CANCELLED
```

## 13. 日志要求
每个任务至少记录：

- job created
- provider request started
- provider response received
- markdown parse started/finished
- static check started/finished
- render process started
- render process exited
- artifact check started/finished
- final state

日志不得包含：

- API Key。
- Authorization header。
- 完整 Provider secret。

## 14. 验收标准
- 无代码块时任务 failed，错误为 `E_LLM_OUTPUT_INVALID`。
- 多代码块时任务 failed，错误为 `E_LLM_OUTPUT_INVALID`。
- ManimGL 代码任务 failed，错误为 `E_STATIC_CHECK_FAILED`。
- 危险 API 代码任务 failed，错误为 `E_STATIC_CHECK_FAILED`。
- Manim 失败时任务 failed，错误为 `E_RENDER_FAIL`。
- MP4 不存在或 duration 为 0 时任务 failed，错误为 `E_ARTIFACT_INVALID`。
- 用户取消 running 渲染时任务进入 `cancelled`，应用不崩溃。
- 失败任务手动重试会创建新 job，不覆盖原 job。
