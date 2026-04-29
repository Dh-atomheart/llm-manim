# DeepSeek + Manim Skill 工作流评估

## 1. 结论
当前设想总体可行，但需要把“大模型调用工具并输出视频”拆成更可控的工程链路：

```text
用户提示词
-> 应用构造系统提示词与 ManimCE 规则上下文
-> DeepSeek API 生成 Markdown 代码块或 tool call 请求
-> 应用校验模型输出
-> 应用写入临时 Manim 脚本
-> 应用调用本地 Manim 渲染
-> 应用收集日志与 MP4
-> 用户预览视频，失败时手动重试
```

核心原则：模型只负责“生成”和“请求”，应用负责“执行”。不要让模型直接决定文件路径、Shell 命令或系统资源操作。

## 2. 可行性评估
### 2.1 DeepSeek API
DeepSeek 官方 API 当前支持 OpenAI 格式和 Anthropic 格式，并列出 `deepseek-v4-flash` 与 `deepseek-v4-pro`。官方模型页显示两者支持 JSON Output 和 Tool Calls，适合作为本项目的 Provider 接入目标。

对本项目的意义：

- 可以用 OpenAI-compatible 方式接入，符合 `docs/spec.md` 的 Provider 设计。
- V1 选择要求模型返回 Markdown Python 代码块，便于直接承载 Manim 源码；该选择解析稳定性弱于 JSON，因此必须做严格代码块解析和静态校验。
- 可以使用 tool/function calling，让模型请求固定工具，例如代码校验或渲染。
- Function Calling 文档明确说明：具体工具功能由用户侧提供，模型本身不执行函数。

### 2.2 Manim Skill
本仓库已有 `references/skills/`：

- `manim-composer`：适合把模糊教学需求拆成场景计划。
- `manimce-best-practices`：适合生成 Manim Community Edition 代码。
- `manimgl-best-practices`：适合 ManimGL/3Blue1Brown 版本，不适合 V1 默认链路。

V1 已在 `docs/spec.md` 中明确使用 Manim Community Edition，因此推荐只把 `manim-composer` 和 `manimce-best-practices` 作为默认规则来源。不要混用 ManimGL 规则，否则容易生成 `from manimlib import *`、`InteractiveScene`、`manimgl` CLI 等不兼容代码。

### 2.3 本地渲染
本地 Manim 渲染可行，但 Windows 上依赖链较长：

- Python/Manim
- FFmpeg
- LaTeX/MiKTeX
- 字体与文本渲染依赖
- 工作区与临时文件权限

这也是为什么渲染必须由应用编排，而不是由模型自由执行命令。

## 3. 推荐工作流
### 3.1 V1 默认链路
V1 推荐采用“受控代码块生成 + 应用执行”：

1. 用户输入自然语言需求。
2. 应用读取精选 ManimCE 规则，而不是把全部 skill 文件塞进上下文。
3. 应用构造系统提示词，明确要求：
   - 只使用 Manim Community Edition。
   - 只使用 `from manim import *`。
   - 输出单个 Scene 类。
   - 不使用网络、文件读写、子进程、随机下载等行为。
   - 只输出一个 Python Markdown 代码块。
4. DeepSeek 返回 Markdown Python 代码块。
5. 应用解析唯一代码块并执行静态校验。
6. 应用把代码写入受控临时目录。
7. 应用用固定命令调用 Manim。
8. 应用检查 MP4 是否存在、可读、非空。
9. 应用记录状态、日志、错误摘要和视频路径。

### 3.2 Tool Calls 的正确用法
可以允许 DeepSeek 请求 tool calls，但工具必须是应用定义的白名单。

推荐 V1 或 V1.1 工具：

```text
validate_manim_code
render_manim_scene
summarize_render_error
```

约束：

- 模型只能传结构化参数，不能传 Shell 命令。
- 应用忽略模型提供的任意绝对路径。
- 应用自己生成工作目录、文件名和渲染命令。
- 未知工具、额外参数、非法参数一律拒绝。
- tool call 失败后不自动进入无限修复循环。

### 3.3 不推荐的链路
不建议 V1 采用“模型代理全流程”：

```text
模型自己规划 -> 自己写文件 -> 自己运行命令 -> 自己修复 -> 自己输出视频
```

原因：

- 难以限制文件系统访问。
- 难以保证命令安全。
- 失败链路不可预测。
- 成本和耗时不可控。
- 与当前 spec 中“失败仅手动重试”的边界冲突。

## 4. 主要困难点
### 4.1 Manim 代码可执行率
LLM 常见失败：

- 混用 ManimCE 和 ManimGL API。
- 使用不存在或旧版本 API。
- Scene 类名和渲染命令不一致。
- LaTeX 语法或转义错误。
- 中文字体不可用。
- 对象过多导致遮挡或渲染过慢。

缓解方式：

- 系统提示词中强制 ManimCE。
- 输出唯一 Python Markdown 代码块，并拒绝无代码块或多代码块输出。
- 静态检查导入、Scene 类、危险 API。
- 建立固定 golden prompts 回归测试。

### 4.2 Tool Call 安全边界
Tool call 不是安全执行许可。模型可能请求：

- 写入任意路径。
- 执行任意命令。
- 读取本地文件。
- 下载外部资源。
- 使用超大分辨率或超长渲染。

缓解方式：

- 工具白名单。
- 参数 schema 校验。
- 固定工作目录。
- 固定渲染命令模板。
- 拒绝路径穿越和绝对路径。
- 渲染队列串行执行。
- 提供取消入口和耗时提醒。

### 4.3 视频质量不可一次性保证
“视频最优化”不能只靠一个系统提示词解决。至少有三层质量：

- 硬质量：能渲染、能播放、MP4 非空。
- 基础视觉质量：无明显空画面、无明显遮挡、节奏不过快。
- 教学质量：推导正确、步骤清晰、符合教师场景。

V1 应把“硬质量”作为自动门槛，把“基础视觉质量”和“教学质量”作为验收样例与人工评估目标。

## 5. 如何提高输出质量
### 5.1 Prompt 分层
推荐分为四层：

1. 产品系统规则：V1 范围、ManimCE、禁止事项、输出格式。
2. ManimCE 精选规则：布局、文本、公式、动画节奏、CLI 约束。
3. 任务需求：用户输入。
4. 输出协议：Markdown Python 代码块格式和示例。

不要把全部 skill 文件一次性注入。应按任务类型选择少量规则：

- 公式推导：`latex`、`text`、`positioning`、`transform-animations`
- 几何变换：`shapes`、`lines`、`animations`、`grouping`
- 物理示意：`updaters`、`timing`、`mobjects`
- 算法可视化：`text`、`grouping`、`animation-groups`

### 5.2 Markdown 代码块输出
V1 推荐模型只输出一个 Python Markdown 代码块：

````markdown
```python
from manim import *

class GeneratedScene(Scene):
    def construct(self):
        title = Text("二次方程求根公式")
        self.play(Write(title))
        self.wait(1)
```
````

应用只信任“存在唯一 Python 代码块”这个外层结构，不信任代码内容；代码仍需静态校验。若没有代码块或出现多个代码块，任务应失败并返回 `E_LLM_OUTPUT_INVALID`。

### 5.3 静态校验
最低校验：

- 必须包含 `from manim import *`。
- 禁止 `from manimlib import *`。
- 必须存在且只渲染一个 Scene 类。
- 禁止 `os.system`、`subprocess`、`requests`、`socket`、`open(`、`eval`、`exec`。
- 禁止模型指定输出路径。
- 限制代码长度和估计场景时长。

### 5.4 渲染后检查
最低检查：

- MP4 文件存在。
- 文件大小大于最小阈值。
- 文件可被视频组件或 ffprobe 读取。
- 时长大于 0。
- Manim 日志无 fatal 错误。

V1 可选检查：

- 抽取首帧，判断是否接近全黑/全白。
- 抽取中间帧，判断是否和首帧完全相同。
- 记录渲染耗时和输出时长比例。

### 5.5 Golden Prompts
建立固定验收提示词：

1. 公式推导：二次方程求根公式推导。
2. 几何变换：三角形旋转、平移和相似变换。
3. 物理示意：匀速圆周运动的速度与向心加速度。
4. 算法可视化：二分查找或冒泡排序。

每次修改系统提示词、skill 注入策略或模型版本，都用这四类样例回归。

## 6. V1 建议边界
保留当前 V1 设定：

- 成功率优先。
- 使用 DeepSeek tool calls 请求渲染，但执行权在应用。
- 失败后手动重试，不自动修复。
- 只使用 Manim Community Edition。
- 用户不查看 Manim 源码。

建议在 V1.1 或 V2 再考虑：

- 一次自动修复。
- 多轮 Agent 自修复。
- 截图抽帧视觉评分。
- 教学质量评分。
- 模板库和可选风格。
- 更强沙箱或隔离执行。

## 7. 对 spec 的建议修订点
后续可把以下内容补入 `docs/spec.md`：

- Provider 支持 DeepSeek OpenAI-compatible API。
- DeepSeek tool calls 只作为请求机制，工具执行由应用控制。
- 生成结果采用唯一 Python Markdown 代码块。
- 渲染命令由应用固定生成，模型不得输出可执行命令。
- Manim skill 默认只使用 `manim-composer` 和 `manimce-best-practices`。
- 新增“静态校验”和“渲染后检查”两段功能需求。

## 8. 参考资料
- DeepSeek Models & Pricing：https://api-docs.deepseek.com/quick_start/pricing
- DeepSeek Function Calling：https://api-docs.deepseek.com/guides/function_calling/
- DeepSeek JSON Output：https://api-docs.deepseek.com/guides/json_mode/
- DeepSeek List Models：https://api-docs.deepseek.com/api/list-models/
- Manim Community 文档：https://docs.manim.community/
- manim_skill：https://github.com/adithya-s-k/manim_skill
- 本仓库 ManimCE skill：`references/skills/manimce-best-practices/SKILL.md`
- 本仓库 Manim composer skill：`references/skills/manim-composer/SKILL.md`
