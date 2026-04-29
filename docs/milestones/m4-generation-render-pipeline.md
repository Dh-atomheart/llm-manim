# M4 Generation & Render Pipeline

## 目标
实现从提示词提交到 MP4 产物入库的后端闭环：Prompt 编排、Provider 调用、Markdown 代码块解析、静态校验、串行队列、半托管 uv/Manim 渲染、硬质量检查、取消和手动重试。

## 范围
包含：

- PromptJob 创建和状态机。
- 串行 Render Queue。
- Prompt/Skill 编排。
- LLM Markdown Python 代码块解析。
- ManimCE 静态校验。
- 半托管 uv/Manim runtime 检查。
- 固定 Manim 渲染命令模板。
- MP4 硬质量检查。
- 取消 running/queued 任务。
- 手动重试创建新任务。

不包含：

- 完整用户生成 UI。
- 视频播放器 UI。
- 自动修复失败脚本。
- 抽帧视觉评分。
- 教学质量自动评分。

## 前置依赖
- M3 Provider Configuration
- `docs/render-pipeline.md`
- `docs/llm-orchestration.md`
- `docs/logging-observability.md`
- `docs/security-boundary.md`
- `docs/database-migrations.md`
- `docs/provider-protocol.md`
- `docs/prompt-contract.md`
- `docs/static-checker.md`
- `docs/runtime-management.md`
- `docs/tech-stack.md`
- `docs/api-contract.md`
- `docs/workspace-storage.md`

## 主要任务
- 通过受控迁移创建 `prompt_jobs`、`render_artifacts`、`job_logs` 表。
- 实现 `submit_prompt_job`。
- 实现 `get_job`。
- 实现 `list_project_jobs`。
- 实现 `cancel_job`。
- 实现 `retry_job`。
- 实现 `get_job_logs`。
- 实现 `get_render_artifact`。
- 实现 `check_runtime`。
- 实现 LLM Orchestrator Service。
- LLM Orchestrator 读取 queued job 和 ProviderConfig。
- LLM Orchestrator 构建 Prompt，注入 ManimCE 精选规则。
- LLM Orchestrator 调用已配置 Provider。
- LLM Orchestrator 提取模型文本并解析唯一 Python Markdown 代码块。
- LLM Orchestrator 调用 Python AST + denylist 静态校验，拒绝无代码块、多代码块、ManimGL、危险 API。
- LLM Orchestrator 写入阶段日志；Provider、Prompt、Parse、Static Check 任一失败都统一落库。
- 阶段日志、脱敏、错误摘要必须符合 `docs/logging-observability.md`。
- 模型输出、脚本写入、路径和命令执行边界必须符合 `docs/security-boundary.md`。
- 将通过校验的代码写入 `jobs/{job_id}/generated_scene.py`。
- 使用 Rust `tokio::process` 和固定命令模板运行 ManimCE。
- 收集 stdout/stderr。
- 检查 MP4 存在、非空、duration > 0、日志无 fatal。

## UI 数据契约
M4 不实现 UI，但后端输出必须支撑 `references/b_pos8dDmvcka/components/views/workbench.tsx` 所表达的工作台体验：

- `get_job` 必须提供 queued/running/succeeded/failed/cancelled 状态、耗时相关时间戳、错误码、错误摘要和建议动作。
- `get_job_logs` 必须提供可展开日志面板所需的阶段、级别、消息和时间。
- `get_render_artifact` 和 `get_video_file_url` 必须支撑成功后的应用内视频预览。
- `cancel_job` 必须支撑 queued/running 状态的取消按钮。
- `retry_job` 必须支撑 failed/cancelled 状态的重试按钮。
- 前端不得为了适配 UI 自行猜测任务终态、拼接本地路径或解析原始渲染日志。

## 接口与数据影响
Tauri command：

- `submit_prompt_job`
- `get_job`
- `list_project_jobs`
- `cancel_job`
- `retry_job`
- `get_job_logs`
- `get_render_artifact`
- `check_runtime`

SQLite 表：

- `prompt_jobs`
- `render_artifacts`
- `job_logs`

文件目录：

```text
jobs/{job_id}/generated_scene.py
jobs/{job_id}/render_stdout.log
jobs/{job_id}/render_stderr.log
jobs/{job_id}/manim.cfg
artifacts/{project_id}/{job_id}/output.mp4
```

错误码：

- `E_LLM_OUTPUT_INVALID`
- `E_STATIC_CHECK_FAILED`
- `E_RENDER_FAIL`
- `E_ARTIFACT_INVALID`
- `E_DEP_MISSING`
- `E_CANCELLED`
- `E_CANCEL_FAILED`
- `E_JOB_NOT_RETRYABLE`
- `E_JOB_NOT_CANCELLABLE`

## 验收标准
- 提交任务后状态为 `queued`。
- 队列取任务后状态为 `running`。
- Orchestrator 成功路径可从 mock Provider 到静态校验通过，并交给渲染阶段。
- Provider/Prompt/Parse/Static Check 任一失败都不会进入 Manim 渲染。
- Job/Artifact/Log 表通过 migration 可重复、可诊断地创建。
- 任何未通过静态校验的代码不得写入可执行渲染脚本。
- 无 Markdown 代码块时任务 `failed`，错误为 `E_LLM_OUTPUT_INVALID`。
- 多 Markdown 代码块时任务 `failed`，错误为 `E_LLM_OUTPUT_INVALID`。
- ManimGL 代码任务 `failed`，错误为 `E_STATIC_CHECK_FAILED`。
- 危险 API 代码任务 `failed`，错误为 `E_STATIC_CHECK_FAILED`。
- Manim 渲染失败时任务 `failed`，错误为 `E_RENDER_FAIL`。
- MP4 硬质量失败时任务 `failed`，错误为 `E_ARTIFACT_INVALID`。
- 成功任务生成 `RenderArtifact`。
- running 任务可取消，应用不崩溃。
- failed/cancelled 任务可手动重试，并创建新 job。
- M4 输出的数据足够 M5 对接参考工作台 UI，不需要前端猜测状态、拼接路径或直接读取文件。
- M4 不引入 LangChain、LangGraph 或类似 agent 框架。

## 风险与处理
- Provider 输出不稳定：严格解析唯一代码块，失败即进入 `E_LLM_OUTPUT_INVALID`。
- 静态校验漏判：V1 使用 Python AST + denylist，并用危险关键字和 ManimGL 样例做回归测试。
- Windows 子进程取消复杂：必须跟踪 Manim 子进程句柄，取消失败返回 `E_CANCEL_FAILED`。
- Runtime 不可用：`check_runtime` 必须提前暴露缺失依赖。
