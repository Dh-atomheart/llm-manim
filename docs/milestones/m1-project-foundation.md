# M1 Project Foundation

## 目标
建立可运行的 Tauri + React + TypeScript + npm 项目骨架，并验证 React 前端可以通过 Tauri command 调用 Rust 后端。M1 只证明工程基础成立，不实现业务闭环。

## 范围
包含：

- 在仓库根目录初始化 Tauri + React + TypeScript 项目。
- 建立基础开发命令。
- 建立 Rust/React 通信样例。
- 建立统一 `{ ok, data, error }` 返回格式雏形。
- 保留已有 `docs/`。

不包含：

- 工作区选择。
- SQLite schema。
- Provider 配置。
- Manim 渲染。
- 真实业务 UI。

## 前置依赖
- `docs/initialization.md`
- `docs/tech-stack.md`
- `docs/frontend-implementation.md`
- `docs/architecture.md`
- `docs/api-contract.md`
- `docs/ui-design.md`
- `docs/ui-wireframes.md`
- `references/b_pos8dDmvcka`

## 主要任务
- 执行根目录 Tauri 脚手架初始化，使用 React + TypeScript + npm。
- 确认 `package.json` 中存在开发、构建、Tauri 启动脚本。
- 建立最小 React 页面，用于显示应用启动状态。
- 最小页面不是自由设计，应抽取 `references/b_pos8dDmvcka` 的基础外观：顶部栏/应用身份、黑白灰、细边框、紧凑间距。
- 最小页面应采用 `docs/ui-design.md` 和 `docs/ui-wireframes.md` 的基础风格：黑白灰、线条化、无阴影和无装饰性渐变。
- 建立一个 Rust command，例如 `ping_backend`，返回统一响应结构。
- 在前端调用该 command，并展示响应。
- 建立前端 command client 雏形，页面不得直接散落 `invoke(...)`。
- 保持前端不直接访问本地文件、不直接执行命令。
- 为后续模块预留目录结构：前端 command client、Rust command layer、domain services。

## 接口与数据影响
Tauri command：

- `ping_backend`：仅 M1 验证用，后续可删除或保留为诊断命令。

返回格式：

```json
{
  "ok": true,
  "data": {
    "message": "pong"
  }
}
```

数据影响：

- 不创建正式 SQLite 数据库。
- 不创建正式工作区。

## 验收标准
- `npm.cmd install` 成功。
- `npm.cmd run tauri dev` 可启动桌面窗口。
- React 页面能调用 Rust command 并显示成功结果。
- command client 能统一处理 `{ ok, data, error }` 响应。
- 页面视觉不引入立体卡片、玻璃、渐变或高饱和彩色装饰。
- 最小页面视觉能自然演进到 `references/b_pos8dDmvcka` 的顶部栏、侧边栏和工作台结构。
- 不引入 Next.js runtime、Tailwind 默认样式体系或整包 shadcn/Radix 组件库。
- `npm.cmd run build` 成功。
- `src-tauri` 下 Rust 编译检查通过。
- 现有 `docs/` 未被脚手架覆盖。

## 风险与处理
- PowerShell 阻止 `npm.ps1`：统一使用 `npm.cmd`。
- 根目录非空导致脚手架拒绝：按 `docs/initialization.md` 保留 `docs/` 并在当前目录初始化。
- Rust/WebView2/Build Tools 缺失：按初始化手册修复环境。
