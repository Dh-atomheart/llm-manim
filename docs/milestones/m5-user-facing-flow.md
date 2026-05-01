# M5 User-Facing Flow

## 目标
打通用户可操作的完整前端闭环：项目详情、生成面板、任务状态、日志面板、视频预览、取消和重试。M5 完成后，用户可以不看源码、不接触命令行，从提示词生成并回看 MP4。

## 范围
包含：

- 项目详情页。
- 提示词输入和 Provider 选择。
- 生成按钮。
- 任务列表和任务详情。
- queued/running/succeeded/failed/cancelled 状态展示。
- 简化日志面板。
- 视频预览。
- 取消 running/queued 任务。
- failed/cancelled 任务手动重试。
- 用户可读错误摘要和下一步建议展示。

不包含：

- 用户查看或复制 Manim 源码。
- 模板库。
- 多轮对话。
- 自动修复。
- 抽帧评分 UI。

## 前置依赖
- M4 Generation & Render Pipeline
- `docs/frontend-implementation.md`
- `docs/logging-observability.md`
- `docs/api-contract.md`
- `docs/spec.md`
- `docs/ui-design.md`
- `docs/ui-wireframes.md`
- `docs/tech-stack.md`
- `references/b_pos8dDmvcka`

## 主要任务
- 实现项目详情页面。
- 以 `references/b_pos8dDmvcka` 为 UI 验收参考，复用其页面结构、视觉层级和核心交互。
- 页面映射必须清晰：
  - `app/page.tsx`：顶部栏、左侧栏、项目列表、Provider 快捷选择和视图切换。
  - `components/views/first-launch.tsx`：首次启动和环境检查。
  - `components/views/workbench.tsx`：提示词输入、任务状态、日志展开、视频预览、取消和重试。
  - `components/views/history.tsx`：历史记录。
  - `components/views/provider-settings.tsx`：Provider 设置。
  - `components/views/basic-settings.tsx`：基础设置。
- 按 `docs/ui-wireframes.md` 落地首启、项目、生成、历史、日志、预览和设置的页面结构。
- 实现生成表单：提示词输入、Provider 选择、模型展示。
- 按 `docs/ui-design.md` 实现黑白灰线条化界面；彩色只用于状态、错误、成功、警告和关键强调。
- 将引用设计中的 Tailwind class 翻译为 CSS Modules；不得引入 Next.js runtime 或整包 shadcn/Radix 组件库。
- 按需重建轻量组件：Button、Input、Textarea、Select、Dialog、StatusBadge、LogPanel、VideoPreview。
- 按 `docs/frontend-implementation.md` 建立页面、store、command client 和 CSS Modules 组织方式。
- 调用 `submit_prompt_job` 创建任务。
- 周期性查询或事件驱动更新 `get_job`。
- 展示任务状态和阶段信息。
- 调用 `get_job_logs` 展示简化日志。
- 日志展示必须符合 `docs/logging-observability.md`：默认简化、开发者详情折叠且脱敏。
- 成功后调用 `get_render_artifact` 和 `get_video_file_url` 获取预览地址。
- 通过 `open_render_artifact` 打开或定位文件，不直接操作本地路径。
- 实现视频播放、暂停和进度拖动。
- 实现取消按钮，调用 `cancel_job`。
- 实现重试按钮，调用 `retry_job`。
- 前端不直接访问工作区文件，不执行 shell，不拼接本地路径。
- 日志、任务列表、视频预览以分隔线和稳定布局组织，不使用卡片堆叠或立体阴影。
- 移除引用设计中的 mock 数据、演示按钮、演示日志、模拟失败逻辑和硬编码路径，全部替换为 Tauri command 数据流。

## 接口与数据影响
Tauri command：

- `list_projects`
- `list_provider_configs`
- `submit_prompt_job`
- `get_job`
- `list_project_jobs`
- `cancel_job`
- `retry_job`
- `get_job_logs`
- `get_render_artifact`
- `get_video_file_url`
- `open_render_artifact`

前端状态：

- 当前项目。
- 当前 Provider。
- 当前任务列表。
- 当前选中任务。
- 视频预览 URL。
- 错误提示与日志展开状态。

## 验收标准
- 用户可进入项目详情页。
- 用户可输入提示词并选择 Provider。
- 点击生成后看到任务进入 queued/running。
- 成功后可在应用内播放 MP4。
- 成功后可通过 `open_render_artifact` 定位或打开产物，前端不直接拼接本地路径。
- 失败后可看到错误摘要、错误码和建议动作。
- running/queued 任务可取消。
- failed/cancelled 任务可重试。
- 任务历史可回看，刷新页面后状态仍来自后端/SQLite。
- runtime broken/missing 时，生成按钮禁用，并能在基础设置页看到 Python、uv、Manim CE、`uv run manim`、FFmpeg、FFprobe 的逐项状态。
- 真实桌面端手工验收需覆盖：无 Provider、runtime broken、提交任务、轮询状态、取消、重试、成功预览、artifact 打开、失败日志展示和窄屏布局。
- 所有用户可见中文文案必须按 UTF-8 正常显示；不得出现 mojibake、替换字符或控制台编码伪影。
- 前端只通过 command client 访问后端，不直接散落 `invoke(...)`。
- 前端不展示 API Key。
- UI 符合黑白灰主色、线条组件、彩色语义提示的规范；所有状态色均配有文字或图标说明。
- 与 `references/b_pos8dDmvcka` 对照检查：顶部栏、左侧项目栏、工作台、日志展开、Provider 选择、首次启动体验和设置页结构保持一致。
- 逐页对照参考 UI：全局布局、工作台、历史、Provider 设置、基础设置、日志展开、状态徽标和视频预览均无明显偏离。
- 不出现营销页、卡片堆叠页、复杂多步骤流程或与参考 UI 明显不一致的布局改写。

## 风险与处理
- 状态轮询过频：设置合理刷新间隔或后续使用事件推送。
- 错误详情过多：默认展示简化错误，开发者详情折叠展示且脱敏。
- 视频 URL 失效：通过 `get_video_file_url` 获取，不直接拼接本地路径。
- UI 越权访问文件：所有文件访问必须经过 Tauri command。
- 视觉风格漂移：以 `references/b_pos8dDmvcka` 和 `docs/ui-design.md` 为验收依据，拒绝拟物、立体、玻璃、渐变和大面积彩色装饰。
- 技术栈漂移：复用设计与交互，不直接采用 Next.js/Tailwind/Radix 全套实现。
