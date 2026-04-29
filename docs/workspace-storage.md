# LLM-Manim V1 工作区与存储规格

## 1. 目标
本文定义 V1 工作区目录结构、SQLite 元数据、明文 API Key 存储、日志脱敏、删除策略与临时文件清理规则。

## 2. 工作区目录结构
用户首次启动选择一个全局工作区目录。应用必须在该目录下创建：

```text
workspace/
  config/
    workspace.json
  db/
    app.sqlite
  projects/
    {project_id}/
      project.json
  jobs/
    {job_id}/
      generated_scene.py
      render_stdout.log
      render_stderr.log
      manim.cfg
  artifacts/
    {project_id}/
      {job_id}/
        output.mp4
  logs/
    app.log
  temp/
  .runtime/
    uv/
    python/
    cache/
```

规则：

- `db/app.sqlite` 是元数据权威来源。
- 文件路径在 SQLite 中优先保存 workspace 相对路径。
- `jobs/{job_id}` 保存任务中间文件和原始渲染日志。
- `artifacts/{project_id}/{job_id}` 保存用户可预览产物。
- `.runtime` 保存半托管 uv/Python/Manim 运行时文件。
- `temp/` 可在应用启动时清理过期内容。

## 3. SQLite 表
### 3.1 `workspace_config`
```text
id TEXT PRIMARY KEY
workspace_path TEXT NOT NULL
schema_version INTEGER NOT NULL
created_at TEXT NOT NULL
updated_at TEXT NOT NULL
```

约束：

- V1 只允许一条有效 workspace config。

### 3.2 `projects`
```text
id TEXT PRIMARY KEY
name TEXT NOT NULL
created_at TEXT NOT NULL
updated_at TEXT NOT NULL
deleted_at TEXT NULL
```

索引：

- `idx_projects_deleted_at`

规则：

- 删除项目优先使用软删除。
- 若实现物理删除，必须同时删除关联 job、artifact、log 索引和文件。

### 3.3 `provider_configs`
```text
id TEXT PRIMARY KEY
provider_type TEXT NOT NULL
base_url TEXT NOT NULL
api_key TEXT NOT NULL
model TEXT NOT NULL
created_at TEXT NOT NULL
updated_at TEXT NOT NULL
deleted_at TEXT NULL
```

约束：

- `provider_type` 只能是 `openai_compatible` 或 `anthropic_compatible`。
- DeepSeek 使用 `openai_compatible`。
- `api_key` V1 明文存储。

安全规则：

- API Key 不得写入 `logs/app.log`。
- API Key 不得写入 `JobLog.message`。
- API Key 不得出现在前端 command 响应中，除非是 save/test 请求由前端传入。

### 3.4 `prompt_jobs`
```text
id TEXT PRIMARY KEY
project_id TEXT NOT NULL
provider_id TEXT NOT NULL
retry_of_job_id TEXT NULL
prompt_text TEXT NOT NULL
state TEXT NOT NULL
error_code TEXT NULL
error_summary TEXT NULL
suggestion TEXT NULL
created_at TEXT NOT NULL
started_at TEXT NULL
finished_at TEXT NULL
```

索引：

- `idx_prompt_jobs_project_id`
- `idx_prompt_jobs_provider_id`
- `idx_prompt_jobs_state`
- `idx_prompt_jobs_created_at`

约束：

- `state` 只能是 `queued`、`running`、`succeeded`、`failed`、`cancelled`。
- `project_id` 引用 `projects.id`。
- `provider_id` 引用 `provider_configs.id`。

### 3.5 `render_artifacts`
```text
id TEXT PRIMARY KEY
job_id TEXT NOT NULL UNIQUE
video_path TEXT NOT NULL
resolution TEXT NOT NULL
fps INTEGER NOT NULL
duration REAL NOT NULL
file_size_bytes INTEGER NOT NULL
created_at TEXT NOT NULL
```

索引：

- `idx_render_artifacts_job_id`

规则：

- `video_path` 保存 workspace 相对路径。
- `duration` 必须大于 0。
- `file_size_bytes` 必须大于最小阈值。

### 3.6 `job_logs`
```text
id TEXT PRIMARY KEY
job_id TEXT NOT NULL
stage TEXT NOT NULL
level TEXT NOT NULL
message TEXT NOT NULL
timestamp TEXT NOT NULL
```

索引：

- `idx_job_logs_job_id`
- `idx_job_logs_level`
- `idx_job_logs_timestamp`

规则：

- `message` 必须经过密钥脱敏。
- 大型 stdout/stderr 原文保存在 `jobs/{job_id}/render_stdout.log` 和 `render_stderr.log`；`job_logs` 只保存摘要与关键阶段。

## 4. ID 与时间格式
- ID 使用稳定字符串，例如 `project_{uuid}`、`job_{uuid}`。
- 时间使用 UTC ISO 8601 字符串。
- 文件夹名使用 ID，不使用用户输入的项目名。

## 5. 工作区初始化
`initialize_workspace` 必须：

1. 验证路径存在或可创建。
2. 验证目录可读写。
3. 创建标准目录结构。
4. 创建或打开 `db/app.sqlite`。
5. 执行 schema migration。
6. 写入 `config/workspace.json`。
7. 返回 workspace 状态。

## 6. 运行时目录
`.runtime` 第一阶段采用半托管 uv 环境：

- 应用负责检查 uv、Python、Manim、FFmpeg、LaTeX 是否可用。
- 应用可在 `.runtime` 下缓存或管理环境。
- V1 不要求完整离线内置 Python/Manim/MiKTeX。

Runtime 状态值：

```text
missing | initializing | ready | broken
```

## 7. 删除策略
### 7.1 删除项目
允许删除：

- 项目没有 queued/running 任务。

拒绝删除：

- 项目存在 queued/running 任务。

删除行为：

- 标记 `projects.deleted_at`。
- 删除或隐藏关联任务、产物和日志。
- 物理文件删除失败时必须返回 `E_IO`，并保留可恢复记录。

### 7.2 删除 Provider
允许删除：

- 没有 queued/running 任务引用该 Provider。

删除行为：

- 标记 `provider_configs.deleted_at`。
- 不删除历史任务中的 provider_id。

### 7.3 删除任务
V1 不提供单独删除任务命令。任务随项目删除。

## 8. 临时文件清理
应用启动时可清理：

- `temp/` 中超过 24 小时的文件。
- 不属于任何 queued/running job 的孤立临时目录。

不得清理：

- `jobs/{job_id}` 中仍处于 queued/running 的任务文件。
- `artifacts/` 中已入库的 MP4。
- `.runtime/` 文件，除非用户明确执行修复/重建环境。

## 9. 日志脱敏
必须脱敏：

- API Key。
- Authorization header。
- Provider 请求体中的 secret 字段。

脱敏格式：

```text
sk-***REDACTED***
```

日志原则：

- 用户界面显示简化日志。
- 开发者展开区可显示更多 details，但仍不得显示 API Key。
- 渲染 stdout/stderr 不应包含 API Key；写入前仍需经过通用脱敏器。

## 10. 备份与迁移
V1 最低要求：

- SQLite schema 必须有 `schema_version`。
- 迁移失败时返回 `E_DB`。
- 不做自动云备份。
- 不做跨设备同步。

## 11. 验收标准
- 新工作区初始化后标准目录全部存在。
- SQLite 表可创建并完成一次读写。
- 保存 Provider 后数据库中有明文 API Key，但 command 列表接口不返回 Key。
- 创建任务后能定位 job 目录、artifact 目录和日志。
- 删除项目时 queued/running 任务会阻止删除。
- 日志脱敏测试不得泄露 API Key。
