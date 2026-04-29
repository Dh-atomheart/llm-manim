# LLM-Manim V1 API 合同

## 1. 目标
本文定义 React 前端与 Tauri/Rust 后端之间的 command 合同。所有 command 必须使用统一返回格式，不允许按命令自由定义错误结构。

## 2. 统一返回格式
### 2.1 成功
```json
{
  "ok": true,
  "data": {}
}
```

### 2.2 失败
```json
{
  "ok": false,
  "error": {
    "code": "E_WORKSPACE_INVALID",
    "message": "工作区不可写",
    "details": {},
    "retryable": false
  }
}
```

字段规则：

- `ok`：必填。
- `data`：成功时必填；无数据时为 `{}`。
- `error`：失败时必填。
- `error.code`：稳定错误码，供前端分支处理。
- `error.message`：用户可读短消息。
- `error.details`：可选对象，不得包含 API Key。
- `error.retryable`：是否适合用户直接重试。

## 3. 通用类型
### 3.1 JobState
```text
queued | running | succeeded | failed | cancelled
```

合法状态迁移：

```text
queued -> running
queued -> cancelled
running -> succeeded
running -> failed
running -> cancelled
failed -> queued     (manual retry creates a new attempt or requeues copied job)
```

禁止：

- `succeeded -> running`
- `cancelled -> running`
- `failed -> succeeded`

### 3.2 ProviderType
```text
openai_compatible | anthropic_compatible
```

DeepSeek 使用 `openai_compatible`。

### 3.3 LogLevel
```text
debug | info | warn | error
```

### 3.4 LogStage
```text
workspace | provider | prompt | llm | parse | static_check | queue | render | artifact | user_action
```

## 4. Workspace Commands
### 4.1 `get_workspace_status`
Request:

```json
{}
```

Success data:

```json
{
  "configured": true,
  "workspace_path": "E:\\Manim4LearnWorkspace",
  "writable": true,
  "database_ready": true,
  "runtime_status": "ready"
}
```

Errors:

- `E_WORKSPACE_INVALID`

State impact: none.

### 4.2 `initialize_workspace`
Request:

```json
{
  "workspace_path": "E:\\Manim4LearnWorkspace"
}
```

Success data:

```json
{
  "workspace_path": "E:\\Manim4LearnWorkspace",
  "created": true,
  "database_ready": true
}
```

Errors:

- `E_WORKSPACE_INVALID`
- `E_IO`
- `E_DB`

State impact:

- Creates standard workspace directories.
- Initializes SQLite database.
- Persists workspace config.

### 4.3 `check_runtime`
Request:

```json
{}
```

Success data:

```json
{
  "status": "ready",
  "python": "ok",
  "uv": "ok",
  "manim": "ok",
  "ffmpeg": "ok",
  "latex": "ok",
  "message": "运行环境可用"
}
```

Errors:

- `E_DEP_MISSING`
- `E_RUNTIME_INVALID`
- `E_IO`

State impact:

- Writes environment check logs.

### 4.4 `repair_runtime`
Request:

```json
{
  "action": "initialize_or_repair"
}
```

Success data:

```json
{
  "status": "ready",
  "message": "运行环境修复完成"
}
```

Errors:

- `E_DEP_MISSING`
- `E_RUNTIME_INVALID`
- `E_IO`
- `E_NET_TIMEOUT`

State impact:

- May create or repair `.runtime` directories.
- May initialize uv environment with locked dependencies.
- Writes runtime repair logs.
- Must not run arbitrary commands supplied by frontend.

## 5. Provider Commands
### 5.1 `list_provider_configs`
Request:

```json
{}
```

Success data:

```json
{
  "providers": [
    {
      "id": "provider_01",
      "provider_type": "openai_compatible",
      "base_url": "https://api.deepseek.com",
      "model": "deepseek-v4-pro",
      "created_at": "2026-04-29T00:00:00Z",
      "updated_at": "2026-04-29T00:00:00Z"
    }
  ]
}
```

API Key must never be returned.

### 5.2 `save_provider_config`
Request:

```json
{
  "id": null,
  "provider_type": "openai_compatible",
  "base_url": "https://api.deepseek.com",
  "api_key": "plain-text-key",
  "model": "deepseek-v4-pro"
}
```

Success data:

```json
{
  "id": "provider_01"
}
```

Errors:

- `E_VALIDATION`
- `E_DB`

State impact:

- Inserts or updates ProviderConfig.
- Stores API Key in plaintext as a V1 decision.
- Must not log `api_key`.

### 5.3 `delete_provider_config`
Request:

```json
{
  "id": "provider_01"
}
```

Success data:

```json
{
  "deleted": true
}
```

Errors:

- `E_NOT_FOUND`
- `E_PROVIDER_IN_USE`
- `E_DB`

State impact:

- Deletes provider only if no active queued/running job uses it.

### 5.4 `test_provider_config`
Request:

```json
{
  "provider_type": "openai_compatible",
  "base_url": "https://api.deepseek.com",
  "api_key": "plain-text-key",
  "model": "deepseek-v4-pro"
}
```

Success data:

```json
{
  "reachable": true,
  "model_accepted": true,
  "message": "连接测试成功"
}
```

Errors:

- `E_AUTH_401`
- `E_NET_TIMEOUT`
- `E_PROVIDER_ERROR`
- `E_PROVIDER_RESPONSE_INVALID`
- `E_VALIDATION`

State impact:

- Writes provider test log.
- Does not persist config unless caller separately invokes `save_provider_config`.

## 6. Project Commands
### 6.1 `list_projects`
Request:

```json
{}
```

Success data:

```json
{
  "projects": [
    {
      "id": "project_01",
      "name": "二次方程动画",
      "created_at": "2026-04-29T00:00:00Z",
      "updated_at": "2026-04-29T00:00:00Z"
    }
  ]
}
```

### 6.2 `create_project`
Request:

```json
{
  "name": "二次方程动画"
}
```

Success data:

```json
{
  "id": "project_01"
}
```

Errors:

- `E_VALIDATION`
- `E_DB`
- `E_IO`

State impact:

- Inserts Project.
- Creates project directory.

### 6.3 `delete_project`
Request:

```json
{
  "id": "project_01"
}
```

Success data:

```json
{
  "deleted": true
}
```

Errors:

- `E_NOT_FOUND`
- `E_PROJECT_HAS_RUNNING_JOB`
- `E_DB`
- `E_IO`

State impact:

- Refuses deletion if project has queued/running jobs.
- Deletes project metadata and associated files for completed/failed/cancelled jobs.

## 7. Job Commands
### 7.1 `submit_prompt_job`
Request:

```json
{
  "project_id": "project_01",
  "provider_id": "provider_01",
  "prompt_text": "用动画解释二次方程求根公式"
}
```

Success data:

```json
{
  "job_id": "job_01",
  "state": "queued"
}
```

Errors:

- `E_WORKSPACE_INVALID`
- `E_NOT_FOUND`
- `E_VALIDATION`
- `E_DB`

State impact:

- Inserts PromptJob as queued.
- Appends initial JobLog.
- Enqueues job.

### 7.2 `get_job`
Request:

```json
{
  "job_id": "job_01"
}
```

Success data:

```json
{
  "id": "job_01",
  "project_id": "project_01",
  "provider_id": "provider_01",
  "state": "running",
  "error_code": null,
  "error_summary": null,
  "suggestion": null,
  "created_at": "2026-04-29T00:00:00Z",
  "started_at": "2026-04-29T00:00:01Z",
  "finished_at": null
}
```

### 7.3 `list_project_jobs`
Request:

```json
{
  "project_id": "project_01"
}
```

Success data:

```json
{
  "jobs": []
}
```

### 7.4 `cancel_job`
Request:

```json
{
  "job_id": "job_01"
}
```

Success data:

```json
{
  "job_id": "job_01",
  "state": "cancelled"
}
```

Errors:

- `E_NOT_FOUND`
- `E_JOB_NOT_CANCELLABLE`
- `E_CANCEL_FAILED`

State impact:

- queued job: mark cancelled.
- running job: terminate render process if active, mark cancelled, write log.

### 7.5 `retry_job`
Request:

```json
{
  "job_id": "job_01"
}
```

Success data:

```json
{
  "job_id": "job_02",
  "state": "queued",
  "retry_of_job_id": "job_01"
}
```

Errors:

- `E_NOT_FOUND`
- `E_JOB_NOT_RETRYABLE`
- `E_DB`

State impact:

- Creates a new queued job copying project_id, provider_id and prompt_text.
- Original job remains unchanged.

## 8. Log and Artifact Commands
### 8.1 `get_job_logs`
Request:

```json
{
  "job_id": "job_01",
  "level": null
}
```

Success data:

```json
{
  "logs": [
    {
      "id": "log_01",
      "stage": "render",
      "level": "info",
      "message": "Manim render started",
      "timestamp": "2026-04-29T00:00:00Z"
    }
  ]
}
```

API Key and raw provider secrets must be redacted.

### 8.2 `get_render_artifact`
Request:

```json
{
  "job_id": "job_01"
}
```

Success data:

```json
{
  "artifact": {
    "id": "artifact_01",
    "job_id": "job_01",
    "video_path": "artifacts/project_01/job_01/output.mp4",
    "resolution": "720p",
    "fps": 30,
    "duration": 42.5,
    "created_at": "2026-04-29T00:00:00Z"
  }
}
```

### 8.3 `get_video_file_url`
Request:

```json
{
  "artifact_id": "artifact_01"
}
```

Success data:

```json
{
  "url": "tauri-asset-url"
}
```

Errors:

- `E_NOT_FOUND`
- `E_ARTIFACT_INVALID`
- `E_IO`

State impact: none.

### 8.4 `open_render_artifact`
Request:

```json
{
  "artifact_id": "artifact_01",
  "mode": "open_file"
}
```

`mode`:

```text
open_file | reveal_in_folder
```

Success data:

```json
{
  "opened": true
}
```

Errors:

- `E_NOT_FOUND`
- `E_ARTIFACT_INVALID`
- `E_IO`

State impact:

- Validates artifact ownership and path under workspace.
- Opens file or reveals it through controlled opener behavior.
- Never accepts arbitrary frontend paths.

## 9. Error Codes
```text
E_AUTH_401
E_ARTIFACT_INVALID
E_CANCEL_FAILED
E_CANCELLED
E_DB
E_DEP_MISSING
E_IO
E_JOB_NOT_CANCELLABLE
E_JOB_NOT_RETRYABLE
E_LLM_OUTPUT_INVALID
E_NET_TIMEOUT
E_NOT_FOUND
E_PROJECT_HAS_RUNNING_JOB
E_PROVIDER_ERROR
E_PROVIDER_RESPONSE_INVALID
E_PROVIDER_IN_USE
E_RENDER_FAIL
E_RUNTIME_INVALID
E_STATIC_CHECK_FAILED
E_VALIDATION
E_WORKSPACE_INVALID
```

## 10. Frontend Handling Rules
- Never infer terminal job success from logs alone; use `get_job`.
- Show `error.message` to users.
- Show `error.details` only in developer/expanded panels after redaction.
- Treat `retryable: true` as permission to show retry CTA.
- Do not display or persist API Key after save.
