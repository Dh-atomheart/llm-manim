# LLM-Manim V1 UI 文本线框

## 1. 目标
本文定义 V1 页面结构。视觉风格以 [ui-design.md](ui-design.md) 为准：黑白灰、线条组件、彩色只作语义提示，无立体阴影、玻璃、渐变或装饰性视觉。

V1 页面结构应映射并复用 `references/b_pos8dDmvcka` 的设计：

- `app/page.tsx`：全局布局、顶部栏、侧边栏、项目列表、Provider 快捷选择和视图切换。
- `components/views/first-launch.tsx`：首次启动、工作区选择和环境检查。
- `components/views/workbench.tsx`：提示词输入、生成状态、视频预览、日志面板、取消和重试入口。
- `components/views/history.tsx`：历史记录。
- `components/views/provider-settings.tsx`：Provider 设置。
- `components/views/basic-settings.tsx`：基础设置。

引用代码中的 mock 数据、演示按钮、演示日志、模拟失败逻辑和硬编码路径只用于说明交互，不得作为产品行为保留。实现时必须替换为 Tauri command、SQLite 状态和真实任务数据。

## 2. 全局结构
```text
顶部栏：应用身份 / 当前项目 / Provider 选择 / 历史与设置入口
左侧栏：新建项目 / 全部视频 / 项目列表 / Provider 设置
主区域：工作台 / 历史 / Provider 设置 / 基础设置
```

规则：

- 一级操作不超过 3 个同时出现。
- 状态用文字 + 小面积语义色标表达。
- 不使用卡片堆叠作为主要布局。
- 分隔线、表格、简洁表单优先。
- 布局、间距、状态徽标和日志展开方式以 `references/b_pos8dDmvcka/app/page.tsx` 和 `components/views/workbench.tsx` 为基准。

## 3. 首次启动页
```text
标题：选择工作区
说明：项目、视频、日志和配置将保存在此目录

[工作区路径输入，只读]
[选择目录]

环境检查：
Python     未检查/可用/缺失
ManimCE    未检查/可用/缺失
FFmpeg     未检查/可用/缺失
LaTeX      未检查/可用/警告

[检查环境]
[继续]
```

要求：

- 未选择工作区前不能继续。
- runtime 缺失时显示修复建议，但不堆叠复杂引导。
- 结构参考 `references/b_pos8dDmvcka/components/views/first-launch.tsx`，但环境状态必须来自 `check_runtime` 或 `repair_runtime`，不得保留 mock 检查。

## 4. 项目列表页
```text
顶部：项目
[新建项目]

项目表格：
名称 | 最近生成 | 任务数 | 状态 | 操作
```

要求：

- 项目为空时显示简洁空状态和新建按钮。
- 删除项目需要确认。
- 全局项目列表优先放在左侧栏，参考 `references/b_pos8dDmvcka/app/page.tsx`。

## 5. 项目详情 / 生成页
```text
顶部：项目名 / Provider 选择 / 设置入口

主输入区：
[多行提示词输入]
[生成]

当前任务：
状态 | 阶段 | 耗时 | [取消]

结果区：
视频预览
播放 / 暂停 / 进度

下方：
历史任务列表
```

要求：

- 生成入口靠近输入框。
- Provider 未配置时，生成按钮不可用并提示去设置。
- 不显示生成源码。
- 不显示本地绝对路径。
- 工作台结构参考 `references/b_pos8dDmvcka/components/views/workbench.tsx`。
- 引用设计中的“失败演示”按钮不得进入产品实现。
- 引用设计中展示的命令、临时路径和输出路径不得作为普通用户日志原样展示。

## 6. 历史任务详情
```text
任务摘要：
状态 / 时间 / Provider / 模型 / 时长

视频：
[预览]
[打开文件]

日志：
阶段 | 级别 | 消息 | 时间

失败时：
错误码
原因
建议动作
[重试]
```

要求：

- 默认展示简化日志。
- 开发者详情可折叠，但仍需脱敏。
- failed/cancelled 任务显示重试。
- 页面结构参考 `references/b_pos8dDmvcka/components/views/history.tsx`。

## 7. 设置页
### 7.1 Provider 设置
```text
Provider 列表
[新增]

表单：
类型
Base URL
模型
API Key

警告：密钥将明文保存在本机工作区/配置文件中。

[测试连接]
[保存]
[删除]
```

要求：

- API Key 保存后不回显。
- 警告使用小面积警告色和明确文本。
- 页面结构参考 `references/b_pos8dDmvcka/components/views/provider-settings.tsx`。

### 7.2 Runtime 设置
```text
Runtime 状态
uv / Python / ManimCE / FFmpeg / LaTeX

[重新检查]
[修复引导]
```

### 7.3 工作区设置
```text
当前工作区
数据库状态
日志目录
产物目录

[打开工作区]
```

基础设置结构参考 `references/b_pos8dDmvcka/components/views/basic-settings.tsx`。任何打开目录或文件的能力必须通过后端 command 校验，不得让前端直接操作本地路径。

## 8. 状态表达
- queued：灰色文本标签。
- running：蓝色或中性色强调，并显示耗时。
- succeeded：绿色小标记。
- failed：红色小标记，必须有错误码和建议。
- cancelled：灰色小标记。
- warning：黄色/橙色小标记，必须配文字。

状态不得只依赖颜色。
