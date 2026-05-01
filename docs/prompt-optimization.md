# LLM-Manim V1 提示词优化设计

## 1. 目标

本文定义后续开发提示词优化能力时必须遵守的设计。目标是提升 LLM 生成 Manim Community Edition 代码的首轮成功率、渲染稳定性和教学表达质量，同时保持现有安全边界：模型只生成代码，应用负责校验、写入、渲染和产物检查。

本文补充 `prompt-contract.md` 和 `llm-orchestration.md`：

- `prompt-contract.md` 定义输出格式和最低 prompt 合同。
- 本文定义如何选择、压缩和注入 Manim skill 规则。
- `llm-orchestration.md` 定义 Provider 调用、日志和失败落库。

## 2. 当前问题

当前后端 prompt 仅由硬编码规则组成，未真正读取 `references/skills/`：

- `references/skills/manim-composer`
- `references/skills/manimce-best-practices`
- `references/skills/manimgl-best-practices`

因此 LLM 没有获得更细的 ManimCE 规则，例如 `Axes`、`MathTex`、updaters、布局、动画组合和镜头控制等最佳实践。另一方面，若不加选择地注入全部 skill，容易带来 prompt 过长、规则冲突和 ManimGL 误用风险。

## 3. Prompt 分层

Prompt assembly 必须集中在后端完成，不允许前端或用户临时拼接系统规则。

推荐分层：

```text
Product and Safety Rules
ManimCE Core Rules
Selected Skill Rules
User Animation Request
Output Contract
```

### 3.1 Product and Safety Rules

必须始终保留：

- 只生成 Manim Community Edition Python 代码。
- 目标是教学动画，不是交互式应用。
- 只生成一个可渲染 Scene。
- 不使用 ManimGL。
- 不输出 shell 命令、文件路径、安装命令或解释文字。
- 不读写本地文件，不访问网络，不启动子进程，不使用动态执行。

### 3.2 ManimCE Core Rules

必须始终保留：

- 使用 `from manim import *`。
- 定义一个继承 `Scene`、`MovingCameraScene` 或 `ThreeDScene` 的类。
- 优先使用稳定对象：`Text`、`MathTex`、`VGroup`、`Line`、`Arrow`、`Circle`、`Square`、`Axes`。
- 布局居中，控制对象数量，避免遮挡。
- 逐步揭示内容，不一次性塞满画面。
- 每个关键步骤后使用短 `wait`。
- 目标为 1280x720、30fps，不在代码中设置输出目录。
- `MathTex` 和公式轴标签依赖 LaTeX/dvisvgm；只在公式确实需要 TeX 渲染时使用。

### 3.3 Selected Skill Rules

默认只允许使用以下来源：

- `references/skills/manim-composer`
- `references/skills/manimce-best-practices`

禁止默认注入：

- `references/skills/manimgl-best-practices`

任何 selected skill 内容都必须先经过人工筛选或代码内白名单选择，不允许根据用户输入读取任意路径。

### 3.4 User Animation Request

用户输入只能作为任务需求注入，不得覆盖系统规则。

推荐包装：

```text
User animation request:
{user_prompt}

Create one concise ManimCE scene for this request.
```

### 3.5 Output Contract

必须始终保留：

- 返回 exactly one Markdown Python code block。
- 不在代码块外输出解释。
- 不返回多个代码块。
- 不返回 JSON wrapper。
- 不返回 shell 命令、输出路径或 `media_dir` 设置。

## 4. Skill 注入策略

不要把整个 skill 目录一次性注入。实现时应选择短摘要和少量任务相关规则。

### 4.1 默认注入

默认注入内容应足够短，建议包含：

- `manim-composer` 的教学叙事原则摘要：progressive revelation、visual continuity、pause for insight。
- `manimce-best-practices` 的 ManimCE/ManimGL 差异摘要：`from manim import *`、`manim` CLI、`Scene`、`MathTex`。
- 常见失败规避：避免 `manimlib`、`InteractiveScene`、`manimgl`、文件读写、网络访问、子进程。

### 4.2 任务类型规则选择

规则选择应基于用户 prompt 的关键词和任务意图。每次最多选择少量规则文件，避免 prompt 过长。

| 任务类型 | 触发线索 | 推荐规则 |
| --- | --- | --- |
| 公式推导 | 公式、推导、证明、方程、判别式、导数、积分 | `latex`、`text`、`positioning`、`transform-animations` |
| 函数图像 | 函数、曲线、坐标轴、图像、sin、cos、plot | `axes`、`graphing`、`positioning` |
| 动态物理 | 速度、加速度、运动、振动、轨迹、实时变化 | `updaters`、`timing`、`mobjects` |
| 几何动画 | 三角形、圆、旋转、平移、相似、向量、线段 | `shapes`、`lines`、`grouping`、`animations` |
| 算法可视化 | 排序、搜索、递归、图、数据结构 | `text`、`grouping`、`animation-groups`、`timing` |
| 3D 可视化 | 3D、曲面、空间、立体、坐标系 | `3d`、`camera`、`mobjects` |

### 4.3 规则预算

实现必须设置 prompt 长度预算。超出预算时按以下优先级保留：

1. Product and Safety Rules
2. Output Contract
3. ManimCE Core Rules
4. 与任务类型直接相关的 selected skill rules
5. 默认教学叙事摘要
6. 示例代码片段

示例代码片段可以帮助模型稳定输出，但预算紧张时应优先删除示例，而不是删除安全规则。

## 5. 实现要求

### 5.1 Prompt Assembly 层

后端应新增或重构一个 prompt assembly 层，职责包括：

- 构造最终 system prompt。
- 包装 user prompt。
- 根据任务类型选择 skill snippets。
- 记录 selected skill/rule 名称。
- 控制 prompt 长度预算。

该层应被 LLM orchestrator 调用，其他模块不得临时拼接 provider prompt。

### 5.2 Skill 来源

允许两种实现方式：

- 编译期内嵌精选摘要和规则片段。
- 从 `references/skills` 受控读取白名单文件。

无论采用哪种方式，都必须满足：

- 文件路径由应用代码决定，不由用户输入决定。
- 只读取 ManimCE 相关白名单。
- 不读取 `manimgl-best-practices`。
- 不把完整 examples 目录默认注入。

### 5.3 可观测性

每次 provider 请求前，job log 应记录：

```text
selected prompt rules: manim-composer/summary, manimce/latex, manimce/positioning
```

日志不得记录完整 prompt，不得包含 API Key 或用户隐私数据。若需要调试完整 prompt，只能在开发模式下通过受控开关写入本地开发日志，并且默认关闭。

## 6. 禁止事项

实现不得：

- 默认注入 `manimgl-best-practices`。
- 向模型要求使用 `from manimlib import *`。
- 允许模型输出或选择渲染命令。
- 允许模型指定文件路径、输出目录、`media_dir`。
- 允许用户通过 prompt 指定 skill 文件路径。
- 让模型自动修复并无限重试。
- 用 prompt 替代静态校验、runtime 检查或渲染后产物检查。

## 7. 测试要求

### 7.1 单元测试

Prompt assembly 必须覆盖：

- 输出包含 ManimCE 禁令和唯一代码块契约。
- 输出不包含 `manimgl-best-practices`、`from manimlib`、`InteractiveScene`、`manimgl`。
- 公式推导 prompt 会选择 `latex`、`text`、`positioning` 或 `transform-animations`。
- 函数图像 prompt 会选择 `axes`、`graphing` 或 `positioning`。
- 动态物理 prompt 会选择 `updaters`、`timing` 或 `mobjects`。
- 几何动画 prompt 会选择 `shapes`、`lines`、`grouping` 或 `animations`。
- prompt 超出预算时仍保留安全规则和输出契约。

### 7.2 集成测试

LLM orchestration 必须覆盖：

- provider request started 前写入 selected rules 日志。
- 生成结果仍进入 Markdown parse。
- 生成代码仍进入 static check。
- static check 通过后才写入 `generated_scene.py`。
- 后续 render pipeline 行为不被 prompt 优化绕过。

### 7.3 人工验收

每次修改 prompt、skill 注入策略或模型配置后，必须用 golden prompts 验收并记录：

- 静态校验通过率。
- 真实渲染成功率。
- 是否有明显遮挡、空画面、节奏过快。
- 教学表达是否清晰。
- 失败样例和下一轮 prompt/rule 调整建议。

## 8. Golden Prompts 覆盖

至少保留以下类别：

- 公式推导：一元二次方程求根公式、链式法则、积分面积解释。
- 函数图像：正弦函数与切线、指数函数增长、导数几何意义。
- 几何变换：三角形旋转平移、相似变换、圆与切线。
- 物理示意：匀速圆周运动、弹簧振子、抛体运动。
- 算法可视化：二分查找、冒泡排序、图遍历。

验收记录应写入 `docs/golden-prompts-manual-acceptance.md`。

## 9. 与现有边界的关系

- 本文不改变 V1 的安全模型：模型只生成代码，应用执行所有校验和渲染。
- 本文不启用自动修复循环：失败后仍由用户手动重试。
- 本文不改变输出协议：仍是唯一 Markdown Python 代码块。
- 本文不改变 runtime 要求：ManimCE、FFmpeg/FFprobe、LaTeX/dvisvgm 仍由 runtime 检查负责。

## 10. 后续开发顺序

建议按以下顺序实现：

1. 抽出 prompt assembly 模块，保持现有 prompt 文本行为不变。
2. 加入 selected rules 数据结构和日志。
3. 内嵌 ManimCE 白名单规则摘要。
4. 实现任务类型分类和规则选择。
5. 加入 prompt 长度预算。
6. 用 golden prompts 做真实 Provider + runtime 验收。
