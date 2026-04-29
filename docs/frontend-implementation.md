# LLM-Manim V1 前端实现规格

## 1. 目标
本文定义 V1 React 前端的实现边界、目录建议、状态管理、command client、样式组织和参考 UI 迁移规则。

## 2. 技术边界
前端锁定：

- React
- TypeScript
- npm
- Zustand
- CSS Modules
- Tauri JavaScript API

前端不得：

- 直接调用 Provider API。
- 直接访问工作区文件。
- 执行 shell、Manim、uv 或 FFmpeg。
- 拼接本地绝对路径。
- 展示 API Key。
- 展示、复制或导出生成的 Manim 源码。

## 3. 建议目录
```text
src/
  app/
    App.tsx
    routes.ts
  components/
    common/
    layout/
    views/
  features/
    workspace/
    projects/
    providers/
    jobs/
    artifacts/
  stores/
    workspaceStore.ts
    projectStore.ts
    providerStore.ts
    jobStore.ts
    uiStore.ts
  services/
    commandClient.ts
    types.ts
  styles/
    tokens.module.css
    globals.css
```

目录可按脚手架微调，但必须保留清晰边界：视图组件、业务 feature、Zustand store、Tauri command client 分离。

## 4. Command Client
前端必须通过统一 command client 调用 Tauri：

- 封装 `{ ok, data, error }`。
- 统一处理 `error.message`、`error.code`、`retryable`。
- 不在页面组件中直接散落 `invoke(...)`。
- 不把 API Key 写入前端持久状态或调试日志。
- 不接受任意本地路径作为前端可执行动作。

## 5. Zustand Store
建议 store 划分：

- `workspaceStore`：工作区状态、runtime 状态、首次启动状态。
- `projectStore`：项目列表、当前项目。
- `providerStore`：Provider 列表、当前 Provider、连接测试状态。
- `jobStore`：任务列表、当前任务、日志、artifact、轮询/刷新状态。
- `uiStore`：当前视图、弹窗、日志展开、非业务 UI 状态。

原则：

- 后端/SQLite 是任务终态权威来源。
- Zustand 只缓存展示状态和用户当前选择。
- 页面刷新或应用重启后必须通过 command 重新加载状态。

## 6. 样式组织
- 使用 CSS Modules。
- 公共 token 放在 `styles/tokens.module.css` 或等效文件中。
- 组件样式贴近组件保存。
- 不把 Tailwind 作为默认样式体系。
- 不引入 Next.js runtime。
- 不整包搬运 shadcn/Radix 组件库。

## 7. 参考 UI 迁移
`references/b_pos8dDmvcka` 是 UI 设计基准。

迁移规则：

- 复用页面结构、视觉层级、交互意图。
- 将 Tailwind class 翻译为 CSS Modules。
- 只按需重建轻量组件：Button、Input、Textarea、Select、Dialog、StatusBadge、LogPanel、VideoPreview。
- 移除 mock 数据、演示按钮、演示日志、模拟失败逻辑和硬编码路径。
- 所有数据替换为 Tauri command 返回值。

页面映射：

- M1：最小启动页和 command 调用样例。
- M2：首次启动、工作区、项目列表。
- M3：Provider 设置。
- M5：工作台、历史、日志、视频预览、基础设置。

## 8. 刷新与状态流
M5 默认可使用轮询：

- running job：1-2 秒刷新一次 `get_job`。
- 日志面板展开时刷新 `get_job_logs`。
- succeeded 后获取 `get_render_artifact` 和 `get_video_file_url`。

后续可升级事件推送，但 V1 不要求。

## 9. 验收标准
- 页面组件不直接调用 Provider、本地文件或 shell。
- command client 是前端访问后端的唯一入口。
- API Key 保存后不回显、不持久化在前端 store。
- 工作台、历史、Provider 设置、基础设置与 `references/b_pos8dDmvcka` 无明显结构偏离。
- Tailwind/Next.js/shadcn/Radix 不成为 V1 前端运行时依赖。
