# M2 Workspace & Storage

## 目标
实现工作区选择、标准目录结构、SQLite 初始化和基础项目管理。M2 开始建设最小 UI，让用户能完成首次启动、创建项目、查看项目列表和删除项目。

## 范围
包含：

- 工作区初始化与状态检查。
- 标准工作区目录创建。
- SQLite 数据库创建与基础 schema。
- 项目 CRUD 的最小闭环。
- 首次启动页、项目列表页、项目创建/删除 UI。
- 首次启动页和项目列表页遵循线条化黑白灰风格，彩色仅用于可写/不可写、成功/失败等状态。

不包含：

- Provider 配置。
- LLM 调用。
- Manim 渲染。
- 任务队列。
- 视频预览。

## 前置依赖
- M1 Project Foundation
- `docs/workspace-storage.md`
- `docs/database-migrations.md`
- `docs/frontend-implementation.md`
- `docs/api-contract.md`
- `docs/tech-stack.md`
- `docs/ui-design.md`
- `docs/ui-wireframes.md`
- `references/b_pos8dDmvcka`

## 主要任务
- 实现 `initialize_workspace`。
- 实现 `get_workspace_status`。
- 创建 `config/`、`db/`、`projects/`、`jobs/`、`artifacts/`、`logs/`、`temp/`、`.runtime/`。
- 初始化 `db/app.sqlite`。
- 通过受控迁移创建 `workspace_config`、`projects` 表。
- 实现 `create_project`、`list_projects`、`delete_project`。
- 项目目录使用 `project_id`，不使用用户输入名称作为目录名。
- UI 显示工作区是否已配置、是否可写、数据库是否可用。
- UI 支持项目列表、创建项目、删除项目。
- UI 使用细边框、分隔线和留白组织内容，不使用立体阴影或装饰性卡片。
- 首次启动页参考 `references/b_pos8dDmvcka/components/views/first-launch.tsx`，但环境状态必须来自 `get_workspace_status`、`initialize_workspace` 和真实 SQLite 状态。
- 项目列表和左侧栏参考 `references/b_pos8dDmvcka/app/page.tsx`，项目数据必须来自 `list_projects`。
- 引用设计中的 mock 检查、mock 项目和硬编码路径不得保留。

## 接口与数据影响
Tauri command：

- `get_workspace_status`
- `initialize_workspace`
- `create_project`
- `list_projects`
- `delete_project`

SQLite 表：

- `workspace_config`
- `projects`

文件目录：

```text
workspace/
  config/
  db/
  projects/
  jobs/
  artifacts/
  logs/
  temp/
  .runtime/
```

错误码：

- `E_WORKSPACE_INVALID`
- `E_IO`
- `E_DB`
- `E_VALIDATION`
- `E_NOT_FOUND`
- `E_PROJECT_HAS_RUNNING_JOB`

## 验收标准
- 用户首次启动可选择工作区。
- 应用可创建标准工作区目录。
- SQLite 可创建并完成一次读写。
- 基础 schema 通过 migration 创建；重复启动不会重复执行已完成迁移。
- 迁移失败返回 `E_DB`，且不得继续创建项目。
- 用户可创建项目。
- 用户可查看项目列表。
- 用户可删除没有 queued/running 任务的项目。
- 删除项目不会影响工作区根目录和全局配置。
- 页面视觉符合 `docs/ui-design.md`：黑白灰为主，状态色有文字说明。
- 首次启动页与项目列表在布局、间距、状态表达上与 `references/b_pos8dDmvcka` 保持一致。

## 风险与处理
- 工作区不可写：返回 `E_WORKSPACE_INVALID` 并提示用户选择其他目录。
- SQLite 初始化失败：返回 `E_DB`，不得继续创建项目。
- 项目删除误删文件：只能删除工作区内由应用创建的项目相关路径。
