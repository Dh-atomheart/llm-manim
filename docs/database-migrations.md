# LLM-Manim V1 数据库迁移规格

## 1. 目标
本文定义 SQLite schema 版本、迁移命名、执行顺序、事务要求、失败处理和开发期重建策略。表结构以 [workspace-storage.md](workspace-storage.md) 为准，迁移执行策略以本文为准。

## 2. 版本来源
SQLite 必须维护 schema 版本。可使用：

- `workspace_config.schema_version`
- 或单独 `schema_migrations` 表

实现必须选择一种方式并保持一致。V1 推荐使用 `schema_migrations`，同时在 workspace 状态中暴露当前版本摘要。

## 3. 迁移文件
命名：

```text
0001_init_workspace.sql
0002_provider_configs.sql
0003_prompt_jobs.sql
0004_render_artifacts_and_logs.sql
```

规则：

- 迁移按编号升序执行。
- 已执行迁移不得修改内容；需要变更时新增迁移。
- 每个迁移必须可在空数据库上按顺序执行。
- 迁移文件应随应用版本提交。

## 4. 里程碑映射
- M2：创建 workspace、projects、schema 版本记录。
- M3：新增 provider_configs，必要时建立 provider 测试日志相关结构。
- M4：新增 prompt_jobs、render_artifacts、job_logs。
- M6：验证迁移失败、重复执行和旧 workspace 升级场景。

## 5. 执行策略
- 打开 workspace 数据库后先执行迁移。
- 每个迁移必须在事务中执行。
- 迁移成功后记录版本。
- 迁移失败返回 `E_DB`。
- 迁移失败时不得继续写入业务数据。
- 不得在迁移中删除用户 artifact、日志文件或 runtime 文件。

## 6. 开发期重建
开发环境允许删除测试 workspace 后重建数据库。

限制：

- 不得自动删除用户真实 workspace。
- 重建命令或脚本必须显式标记为 dev/test。
- 文档和 UI 不应引导普通用户手动删除数据库。

## 7. 回滚策略
V1 不要求 down migration。

失败处理：

- 当前迁移事务回滚。
- 保留旧版本数据库。
- 返回 `E_DB` 和用户可读说明。
- 建议用户备份 workspace 后重试或选择新 workspace。

## 8. 测试要求
- 空 workspace 能执行全部迁移。
- 已最新 workspace 重复启动不会重复执行迁移。
- 中途失败不会产生部分业务表写入。
- M2/M3/M4 所需表在对应里程碑可用。
- 迁移失败映射为 `E_DB`。
