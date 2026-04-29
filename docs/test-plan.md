# LLM-Manim V1 测试计划

## 1. 目标
本文定义 V1 的测试工具链、测试场景和验收方式。V1 测试重点是稳定闭环、错误可诊断、日志脱敏和 UI 核心路径可用。

## 2. 工具链
锁定：

- Rust 单元测试。
- Rust 集成测试。
- Vitest。
- Playwright。
- mock Provider。
- fake Manim/uv runner。

原则：

- Provider、Manim 和文件系统失败场景必须可控模拟。
- 不依赖真实 API Key 完成自动化测试。
- 不用真实长时间渲染作为常规测试前置。

## 3. 后端测试
覆盖：

- Tauri command 统一返回格式。
- SQLite schema 初始化和迁移。
- SQLite migration 顺序、重复执行、失败回滚和 `E_DB` 映射。
- Provider 配置 CRUD。
- API Key 不出现在列表响应、日志和错误 details。
- 通用日志脱敏器覆盖 Provider、LLM、渲染 stdout/stderr 和错误 details。
- Provider 错误映射。
- LLM Orchestrator 成功路径和失败落库。
- Markdown 代码块解析。
- Python AST 静态校验。
- PromptJob 状态迁移。
- 串行队列。
- 取消 queued/running job。
- 手动重试创建新 job。
- artifact 硬质量检查。
- 安全边界：危险代码不进入渲染、路径限制、artifact 打开校验。

LLM Orchestrator 测试必须覆盖：

- Prompt 构建变量正确注入。
- mock Provider 返回有效代码块后进入静态校验。
- mock Provider 返回无代码块时返回 `E_LLM_OUTPUT_INVALID`。
- mock Provider 返回多代码块时返回 `E_LLM_OUTPUT_INVALID`。
- mock Provider 返回 ManimGL 或危险 API 时返回 `E_STATIC_CHECK_FAILED`。
- Provider 鉴权失败时返回 `E_AUTH_401`。
- Provider 响应结构无效时返回 `E_PROVIDER_RESPONSE_INVALID`。
- 静态校验失败时不写入可执行渲染脚本。
- 任意失败都写入 `PromptJob.failed`、错误码、简化日志和建议动作。

## 4. 前端测试
Vitest 覆盖：

- Zustand store 状态更新。
- command client 错误处理。
- command client 统一处理 `{ ok, data, error }`。
- 首次启动流程状态。
- Provider 表单校验和明文 Key 提示。
- 任务状态展示。
- 日志脱敏展示。

Playwright 覆盖：

- 首次启动：选择工作区 -> 环境状态 -> Provider 设置 -> 新建项目。
- 主流程：输入提示词 -> 生成 -> 任务状态 -> 视频预览。
- 失败流程：Provider 失败、静态校验失败、渲染失败。
- 取消与重试。
- UI 风格检查：无立体阴影、无玻璃、无渐变、大面积色块仅用于语义状态时也要克制。
- 前端边界检查：不直接调用 Provider、不直接读取文件、不展示 API Key 或源码。

## 5. Mock Provider
mock Provider 必须支持：

- 返回有效 ManimCE 代码块。
- 返回无代码块文本。
- 返回多个代码块。
- 返回 ManimGL 代码。
- 返回危险 API 代码。
- 返回 401。
- 返回超时。
- 返回无效响应结构。

## 6. Fake Manim Runner
fake runner 必须支持：

- 成功生成假 MP4 元数据。
- 非零退出。
- 不生成文件。
- 生成空文件。
- 生成 duration 为 0 的结果。
- 长时间运行以测试取消。

## 7. Golden Prompts
四类验收样例：

1. 公式推导：二次方程求根公式。
2. 几何变换：三角形旋转、平移和相似变换。
3. 物理示意：匀速圆周运动速度与向心加速度。
4. 算法可视化：二分查找或冒泡排序。

自动验收只要求：

- 任务最终 succeeded。
- MP4 存在。
- 文件非空。
- duration > 0。
- 日志无 fatal。

教学质量、遮挡、错位和节奏由人工验收记录，不作为每次运行时自动判定。

## 8. 失败场景
必须覆盖：

- Provider 鉴权失败。
- Provider 网络超时。
- Provider 响应结构无效。
- 无 Markdown 代码块。
- 多 Markdown 代码块。
- Python AST 解析失败。
- ManimGL。
- 危险 API。
- runtime 缺失。
- Manim 非零退出。
- MP4 缺失。
- duration 读取失败。
- 用户取消。
- 数据库迁移失败。
- 日志脱敏失败回归样例。

## 9. 发布前检查
- 全部自动化测试通过。
- 四类 golden prompts 至少完成一次人工验收。
- API Key 不出现在 UI、日志、错误详情、测试快照中。
- 文档与实现的 command、错误码、状态机一致。
- 文档与实现的迁移、日志、安全边界、前端 command client 行为一致。
